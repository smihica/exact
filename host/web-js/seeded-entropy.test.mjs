import { test, expect } from 'bun:test';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { open } from '../../scripts/agent.mjs';
import { webCompiler } from './module.mjs';

// LLP 1069.005 D2b on the JS target: under the driver a TypeScript source draws the launch seed's stream, from its
// first answer at boot. Seed 1's first id is the one a native module draws (js/tests/it/entropy.rs).
test('under the driver a source draws the seeded stream: one seed, one id at every launch, and logs say so once', async () => {
  if (webCompiler().cmd === 'cargo') throw new Error('build the compiler first: cargo build -p exact-web-js');
  const root = resolve(import.meta.dir, '../..'), dir = mkdtempSync(resolve(root, 'target/seeded-entropy-test-'));
  let server;
  try {
    writeFileSync(resolve(dir, 'app.json'), JSON.stringify({ name: 'Seeded', app: { id: 'test.seeded', name: 'Seeded' }, host: { web: {} } }));
    writeFileSync(resolve(dir, 'app.contract'), `component Seeded
  state n = 0
  resource id = id(n) as shape string else empty()
  resource bytes = bytes(n) as shape string else empty()
  action again
    n = n + 1
  view
    column
      text id testId="id"
      text bytes testId="bytes"
      button press=again testId="again"
        text "Again"
`);
    writeFileSync(resolve(dir, 'app.ts'), `export const appId = 'test.seeded';
export const grants = '';
export function answer(name: string): any {
  if (name === 'id') return crypto.randomUUID();
  if (name === 'bytes') return Array.from(crypto.getRandomValues(new Uint8Array(4)), b => b.toString(16).padStart(2, '0')).join('');
  throw new Error('no source ' + name);
}
`);
    const build = Bun.spawn([process.execPath, 'host/web-js/build.mjs', 'seeded', '--out', resolve(dir, 'dist'), '--render', 'none'], {
      cwd: root, env: { ...process.env, EXACT_APP_DIR: dir }, stdout: 'pipe', stderr: 'pipe',
    });
    const [status, stdout, stderr] = await Promise.all([build.exited, new Response(build.stdout).text(), new Response(build.stderr).text()]);
    expect({ status, error: status ? stdout + stderr : '' }).toEqual({ status: 0, error: '' });
    server = Bun.serve({ hostname: '127.0.0.1', port: 0, async fetch(req) {
      const path = new URL(req.url).pathname;
      const file = Bun.file(resolve(dir, 'dist', path === '/' ? 'index.html' : path.slice(1)));
      return await file.exists() ? new Response(file) : new Response('not found', { status: 404 });
    } });
    const launch = async seed => {
      const browser = await open({ host: 'web', url: server.url.href, webDist: resolve(dir, 'dist'), seed });
      try {
        const first = (await browser.state()).resources;
        await browser.tap('again');
        const second = (await browser.state()).resources;
        const said = (await browser.logs()).lines.filter(line => line.includes("crypto: the driver seeds this launch's randomness"));
        return { first, second, said };
      } finally { await browser.close(); }
    };
    const a = await launch(), b = await launch(), other = await launch(7);
    expect(a.first.id).toBe('deb201fb-035c-4c32-bbbf-3da08991a485');
    expect(a.first.bytes).toMatch(/^[0-9a-f]{8}$/);
    expect(b.first).toEqual(a.first);
    expect(b.second).toEqual(a.second);
    // The stream goes on: a second ask draws the next bytes.
    expect(a.second.id).not.toBe(a.first.id);
    expect(other.first.id).not.toBe(a.first.id);
    expect(a.said).toEqual([expect.stringContaining('(seed 1)')]);
    expect(other.said).toEqual([expect.stringContaining('(seed 7)')]);
  } finally {
    server?.stop(true); rmSync(dir, { recursive: true, force: true });
  }
}, 120000);
