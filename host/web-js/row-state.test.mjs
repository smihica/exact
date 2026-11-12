import { test, expect } from 'bun:test';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { open } from '../../scripts/agent.mjs';
import { webCompiler } from './module.mjs';

const root = resolve(import.meta.dir, '../..');
const child = `
component Inner
  props
    n: number
  state a = n * 2
  state b = a + 1
  state label = \`\${a} \${b}\`
  action bump
    a = a + 10
  view
    column
      text \`\${a} \${b} \${label}\` testId=\`inner-\${n}\`
      button "bump" press=bump testId=\`bump-\${n}\`
`;

async function fixture(source, run) {
  if (webCompiler().cmd === 'cargo') throw new Error('build the compiler first: cargo build -p exact-web-js');
  mkdirSync(resolve(root, 'target'), { recursive: true });
  const dir = mkdtempSync(resolve(root, 'target/row-state-test-'));
  // open() resolves a path through EXACT_APP_DIR, which is shared across test files.
  const appDir = process.env.EXACT_APP_DIR;
  let browser, server;
  try {
    writeFileSync(resolve(dir, 'app.json'), JSON.stringify({ name: 'Row State', app: { id: 'test.row-state', name: 'Row State' } }));
    writeFileSync(resolve(dir, 'app.contract'), source + child);
    const dist = resolve(dir, 'dist');
    const build = Bun.spawn([process.execPath, 'host/web-js/build.mjs', 'row-state', '--out', dist, '--render', 'none'], {
      cwd: root, env: { ...process.env, EXACT_APP_DIR: dir }, stdout: 'pipe', stderr: 'pipe',
    });
    const [status, stdout, stderr] = await Promise.all([build.exited, new Response(build.stdout).text(), new Response(build.stderr).text()]);
    expect({ status, error: status ? stdout + stderr : '' }).toEqual({ status: 0, error: '' });
    server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(req) {
      const path = new URL(req.url).pathname;
      if (path.startsWith('/shared/')) return new Response(Bun.file(resolve(root, 'host/web', path.slice('/shared/'.length))));
      const file = resolve(dist, path === '/' ? 'index.html' : path.slice(1));
      return existsSync(file) ? new Response(Bun.file(file)) : new Response('Not found', { status: 404 });
    } });
    browser = await open({ host: 'web', app: dir, url: server.url.href });
    const texts = async () => (await browser.tree()).nodes
      .filter(n => n.props.testId?.startsWith('inner-')).map(n => [n.props.testId, n.props.text]);
    await run(browser, texts);
  } finally {
    if (appDir === undefined) delete process.env.EXACT_APP_DIR;
    else process.env.EXACT_APP_DIR = appDir;
    await browser?.close();
    server?.stop(true);
    rmSync(dir, { recursive: true, force: true });
  }
  expect(process.env.EXACT_APP_DIR).toBe(appDir);
}

for (const [name, region] of [
  ['for', 'for n in [value] key=n\n        Inner(n=n)'],
  ['if', 'if value > 0\n        Inner(n=value)'],
  ['match', 'match some(value)\n        case some(n)\n          Inner(n=n)\n        case none\n          text "empty"'],
]) {
  test(`a child's state initializers read earlier state in a root ${name}`, () => fixture(`component App
  state value = 1
  action raise
    value = value + 1
  view
    column
      button "raise" press=raise testId="raise"
      ${region}
`, async (page, texts) => {
    expect(await texts()).toEqual([['inner-1', '2 3 2 3']]);
    await page.tap('bump-1');
    expect(await texts()).toEqual([['inner-1', '12 3 2 3']]);
    await page.tap('raise');
    expect(await texts()).toEqual([['inner-2', name === 'for' ? '4 5 4 5' : '12 3 2 3']]);
  }), 120_000);
}

test('nested child state initializes on boot and on opening, stays with its row key, and resets on reopening', () => fixture(`component App
  state items = [1, 2]
  action reverse
    items = [2, 1]
  view
    column
      button "reverse" press=reverse testId="reverse"
      for i in items key=i
        Outer(n=i)

component Outer
  props
    n: number
  state open = n == 1
  state value = n
  action toggle
    open = not open
  view
    column
      button "toggle" press=toggle testId=\`toggle-\${n}\`
      if open
        Inner(n=value)
`, async (page, texts) => {
  expect(await texts()).toEqual([['inner-1', '2 3 2 3']]);
  await page.tap('toggle-2');
  expect(await texts()).toEqual([['inner-1', '2 3 2 3'], ['inner-2', '4 5 4 5']]);
  await page.tap('bump-1');
  await page.tap('reverse');
  expect(await texts()).toEqual([['inner-2', '4 5 4 5'], ['inner-1', '12 3 2 3']]);
  await page.tap('toggle-1');
  expect(await texts()).toEqual([['inner-2', '4 5 4 5']]);
  await page.tap('toggle-1');
  expect(await texts()).toEqual([['inner-2', '4 5 4 5'], ['inner-1', '2 3 2 3']]);
}), 120_000);
