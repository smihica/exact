import { hostFixtureFile as readFileSync } from './glue-fixture.mjs';
// The manifest's `userInterfaceStyle` on the web: a fixed style is the page's `color-scheme` from its first paint, in
// the shell both web targets write (host/web/look.mjs `withInterfaceStyle`), so the canvas and the browser's own
// controls are in it whatever the browser prefers; `automatic` leaves the shell's `light dark`. A real JS build of an
// app outside the repository (`EXACT_APP_DIR`), served and read in headless Chrome that prefers light; what the app reads as the
// system's scheme is the fixed one (navigation.js `preferences`).
import { test, expect, afterAll } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { cdpBrowser, chromium } from '../../../scripts/agent-launch.mjs';
import { need } from '../../../scripts/coverage.mjs';
import { withInterfaceStyle, withLook } from '../look.mjs';
import { renderer } from '../../web-js/render.mjs';

const ROOT = resolve(fileURLToPath(new URL('../../..', import.meta.url)));
const dir = mkdtempSync(resolve(tmpdir(), 'exact-interface-style-'));
afterAll(() => rmSync(dir, { recursive: true, force: true }));
const check = need('chrome', chromium().unavailable);

test('a fixed style replaces the shell\'s `light dark` and leads the head; none leaves the shell as it is', () => {
  const shell = withLook(readFileSync(resolve(ROOT, 'host/web/index.html'), 'utf8'));
  expect(withInterfaceStyle(shell, null)).toBe(shell);
  const dark = withInterfaceStyle(shell, 'dark');
  expect(dark).toContain('html { color-scheme: dark; }');
  expect(dark).not.toContain('color-scheme: light dark');
  expect(dark).toMatch(/<meta name="viewport"[^>]*>\n<meta name="color-scheme" content="dark">\n/);
});

/** The JS build of a one-field app with `style`: its head, and in Chrome preferring light the html's computed
 * `color-scheme`, the field's background and the system scheme the app reads (`exactViewport()`). */
async function built(style) {
  const app = resolve(dir, style);
  mkdirSync(app, { recursive: true });
  writeFileSync(resolve(app, 'app.json'), JSON.stringify({ name: 'Style', app: { id: 'com.example.style', name: 'Style' }, host: { web: {} }, userInterfaceStyle: style }));
  writeFileSync(resolve(app, 'app.contract'), 'shape SystemMedia\n  prefersColorScheme: string\n\ncomponent App\n  state name: string = ""\n  resource media = exactViewport() as shape SystemMedia\n  view\n    column\n      input value=name aria-label="Name" testId="name"\n      text media.prefersColorScheme testId="system"\n');
  const dist = resolve(app, 'dist');
  const build = spawnSync(process.execPath, ['host/web-js/build.mjs', 'style', '--out', dist, '--render', 'js'], { cwd: ROOT, encoding: 'utf8', env: { ...process.env, EXACT_APP_DIR: app } });
  expect(build.status, build.stderr).toBe(0);
  // A page the JavaScript runtime renders on the server reads the same system scheme.
  const rendered = /data-testid="system"[^>]*>([a-z]+)</.exec((await renderer(dist)('/')).html)?.[1];
  const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' };
  const server = createServer((req, res) => {
    const path = resolve(dist, '.' + (req.url === '/' ? '/index.html' : req.url.split('?')[0]));
    if (!existsSync(path)) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { 'content-type': types[extname(path)] ?? 'application/octet-stream' }); res.end(readFileSync(path));
  });
  await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
  const suite = await cdpBrowser();
  try {
    const { sessionId } = await suite.page();
    const call = (method, params) => suite.cdp.send(method, params, sessionId);
    await call('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }] });
    const evaluate = async (expression) => (await call('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.value;
    await call('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
    const field = `new Promise((ok) => { const read = () => { const f = document.querySelector('[data-testid="name"], input'); f ? ok(getComputedStyle(f).backgroundColor) : setTimeout(read, 20); }; read(); })`;
    const background = await evaluate(field);
    const system = await evaluate(`new Promise((ok) => { const read = () => { const t = document.querySelector('[data-testid="system"]')?.textContent; t ? ok(t) : setTimeout(read, 20); }; read(); })`);
    return { head: readFileSync(resolve(dist, 'index.html'), 'utf8'), scheme: await evaluate('getComputedStyle(document.documentElement).colorScheme'), field: background, system, rendered };
  } finally {
    await suite.close();
    server.close();
  }
}

check('a `dark` app is dark in a browser that prefers light; `automatic` follows the browser', async () => {
  const dark = await built('dark');
  expect(dark.head).toContain('<meta name="color-scheme" content="dark">');
  expect(dark.scheme).toBe('dark');
  // What the app reads as the system's scheme is the fixed one, as on iOS.
  expect(dark.system).toBe('dark');
  expect(dark.rendered).toBe('dark');
  const automatic = await built('automatic');
  expect(automatic.head).not.toContain('name="color-scheme"');
  expect(automatic.scheme).toBe('light dark');
  expect(automatic.system).toBe('light');
  expect(automatic.rendered).toBe('light');
  // The browser's own field is drawn in the page's scheme: dark there, light here.
  expect(dark.field).not.toBe(automatic.field);
}, 240000);
