// A bound `-exact-layout-transition` on the JS runtime (GitHub #809): the shape reduced motion takes, `none` while calm
// and a curve otherwise. A row holding the curve slides when a row above it goes; one whose value is `none` jumps; one
// that gains the curve in the commit that moves it slides from where it was, as presence-glue.js plays a style op that
// gains the row. Build exact-web-js once before running this test (docs/testing.md).
import { test, expect } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { suiteBrowser } from '../../scripts/agent-launch.mjs';
import { withLook } from '../web/look.mjs';

const SOURCE = `component App
  state calm = false
  state shown = true
  action motion
    calm = not calm
  action drop
    shown = not shown
  action both
    calm = not calm
    shown = not shown
  view
    column
      button "Motion" press=motion testId="motion"
      button "Drop" press=drop testId="drop"
      button "Both" press=both testId="both"
      if shown
        text "First" testId="first" height=40
      text "Second" testId="second" height=40 -exact-layout-transition=(calm ? "none" : "2000ms linear")
`;

test('a bound layout transition slides while it holds a curve, jumps at none, and slides from where it was when gained', async () => {
  const root = resolve(import.meta.dir, '../..');
  const target = resolve(root, 'target');
  mkdirSync(target, { recursive: true });
  const dir = mkdtempSync(resolve(target, 'layout-transition-'));
  const style = withLook(readFileSync(resolve(root, 'host/web/index.html'), 'utf8')).match(/<style>([\s\S]*?)<\/style>/)[1];
  let server, browser;
  try {
    writeFileSync(resolve(dir, 'app.contract'), SOURCE);
    const compiled = spawnSync(resolve(target, 'debug/exact-web-js'), ['js', resolve(dir, 'app.contract'), '-o', dir], { encoding: 'utf8' });
    expect(compiled.status, compiled.stderr).toBe(0);
    server = Bun.serve({ port: 0, hostname: '127.0.0.1', fetch(request) {
      const path = new URL(request.url).pathname.slice(1);
      if (!path) return new Response(`<style>${style}</style><link rel="stylesheet" href="/app.css">
        <div id="exact-root"></div><script type="module">import app from '/app.js';app();</script>`,
      { headers: { 'content-type': 'text/html' } });
      // A module that re-exports the host's own reaches it through `../web/`.
      const bases = path.startsWith('web/') ? [resolve(root, 'host')] : [dir, import.meta.dir, resolve(root, 'host/web')];
      const file = bases.map(base => resolve(base, path)).find(existsSync);
      return file ? new Response(Bun.file(file)) : new Response('Not found', { status: 404 });
    } });
    browser = await suiteBrowser();
    const page = await (await browser.newContext({ viewport: { width: 400, height: 700 } })).newPage();
    await page.goto(server.url.href);
    await page.waitForFunction(() => globalThis.exact?.presenceLive);
    // The ops presence hears, as wasm's batch would carry them: one only for a view gaining (before) or losing or gaining (after) the row.
    await page.evaluate(() => {
      const p = globalThis.exact.presenceLive, heard = globalThis.heard = [];
      for (const k of ['before', 'after']) { const f = p[k].bind(p); p[k] = (batch, views) => { heard.push([k, (batch.ops ?? []).length]); return f(batch, views); }; }
    });
    const heard = () => page.evaluate(() => globalThis.heard.splice(0));
    // Each press: the row's declaration, then where it shows now against where it lands, and whether it plays.
    const press = (id) => page.evaluate(async (id) => {
      const second = document.querySelector('[data-testid="second"]');
      document.querySelector(`[data-testid="${id}"]`).click();
      await new Promise((r) => setTimeout(r));
      const shown = second.getBoundingClientRect().top, playing = second.getAnimations().length > 0;
      await Promise.all(second.getAnimations().map((a) => (a.finish(), a.finished)));
      return { css: second.style.getPropertyValue('--exact-layout-transition'), moving: playing && Math.abs(shown - second.getBoundingClientRect().top) > 1 };
    }, id);
    expect(await press('drop')).toEqual({ css: '2000 0 linear', moving: true });
    expect(await heard()).toEqual([['before', 0], ['after', 0]]);
    expect(await press('motion')).toEqual({ css: '', moving: false });
    expect(await heard()).toEqual([['before', 0], ['after', 1]]);
    // Calm: nothing to measure (no op), and it jumps.
    expect(await press('drop')).toEqual({ css: '', moving: false });
    expect(await heard()).toEqual([['before', 0], ['after', 0]]);
    // Gained in the commit that moves it (First goes again).
    expect(await press('both')).toEqual({ css: '2000 0 linear', moving: true });
    expect(await heard()).toEqual([['before', 1], ['after', 1]]);
    // Lost in the commit that moves it: it jumps, and nothing plays it after.
    expect(await press('both')).toEqual({ css: '', moving: false });
    expect(await heard()).toEqual([['before', 0], ['after', 1]]);
    expect(await page.evaluate(() => globalThis.exact.presenceLive.tracking())).toBe(false);
  } finally {
    await browser?.close();
    server?.stop(true);
    rmSync(dir, { recursive: true, force: true });
  }
}, 120000);

test('removed motion views leave the registry after their exit and keep identity when reattached', async () => {
  const root = resolve(import.meta.dir, '../..');
  const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(request) {
    const path = new URL(request.url).pathname.slice(1);
    if (!path) return new Response('<body><div id="exact-root"></div>', { headers: { 'content-type': 'text/html' } });
    if (path === 'names.js') return new Response('export const sourceTypes={};', { headers: { 'content-type': 'text/javascript' } });
    const bases = path.startsWith('web/') ? [resolve(root, 'host')] : [resolve(root, 'host/web-js'), resolve(root, 'host/web')];
    const file = bases.map(base => resolve(base, path)).find(existsSync);
    return file ? new Response(Bun.file(file), { headers: { 'content-type': 'text/javascript' } }) : new Response('', { status: 404 });
  } });
  let browser;
  try {
    browser = await suiteBrowser(); const page = await browser.newPage();
    await page.goto(server.url.href);
    const result = await page.evaluate(async () => {
      globalThis.__exactRender = true;
      const rt = await import('/rt.js');
      const root = document.getElementById('exact-root');
      for (let i = 0; i < 100; i++) {
        const node = document.createElement('section'); root.append(node);
        const scope = rt.scope(() => rt.mo(node, () => [null, null, null, 1, null]));
        node.remove(); rt.end(scope);
      }
      await Promise.resolve(); await Promise.resolve();
      const retired = rt.Views.size;
      const node = document.createElement('section'); root.append(node);
      const scope = rt.scope(() => rt.mo(node, () => [null, null, null, 1, null]));
      const id = rt.viewId(node); rt.end(scope);
      await Promise.resolve();
      const exitKept = rt.Views.get(id) === node; // still connected while its exit plays
      node.remove(); await Promise.resolve(); await Promise.resolve();
      const exitGone = !rt.Views.has(id);
      root.append(node);
      const reattached = rt.viewId(node);
      node.remove(); await Promise.resolve(); await Promise.resolve();
      return { retired, exitKept, exitGone, same: id === reattached, remaining: rt.Views.size };
    });
    expect(result).toEqual({ retired: 0, exitKept: true, exitGone: true, same: true, remaining: 0 });
  } finally { await browser?.close(); server.stop(true); }
}, 30000);

test.each(['chromium', 'webkit'])('computed exit animations follow reduced motion beside fixed and computed press scales on %s', async engine => {
  const app = await exitApp(engine, `shape Media
  prefersReducedMotion: bool

keyframes leave
  to opacity=0 scale=0.75

component App
  resource media = exactViewport() as shape Media
  state shown = true
  action toggle
    shown = not shown
  view
    column
      text (media.prefersReducedMotion ? "reduced" : "full") testId="mode"
      button "Toggle" press=toggle testId="toggle"
      if shown
        box testId="fixed" width=40 height=40 background-color="#334455" press=toggle -exact-press-scale=0.97 -exact-exit-animation=(media.prefersReducedMotion ? "none" : "leave 2000ms linear both")
        box testId="computed" width=40 height=40 background-color="#334455" press=toggle -exact-press-scale=(media.prefersReducedMotion ? 1 : 0.97) -exact-exit-animation=(media.prefersReducedMotion ? "none" : "leave 2000ms linear both")
`, { reducedMotion: 'reduce' });
  try {
    const { page, drop } = app;
    await page.waitForFunction(() => document.querySelector('[data-testid="mode"]')?.textContent === 'reduced');
    expect(await drop('toggle')).toEqual({ ghosts: 0, scales: [], durations: [], remaining: 0 });
    await page.locator('[data-testid="toggle"]').click();
    await page.locator('[data-testid="computed"]').waitFor();
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.waitForFunction(() => document.querySelector('[data-testid="mode"]')?.textContent === 'full');
    expect(await drop('toggle')).toEqual({ ghosts: 2, scales: [0.875, 0.875], durations: [2000, 2000], remaining: 0 });
    await page.locator('[data-testid="toggle"]').click();
    await page.locator('[data-testid="computed"]').waitFor();
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.waitForFunction(() => document.querySelector('[data-testid="mode"]')?.textContent === 'reduced');
    expect(await drop('toggle')).toEqual({ ghosts: 0, scales: [], durations: [], remaining: 0 });
  } finally { await app.close(); }
}, 60000);

// The runner ends an arm without updating its bindings, so a removed node plays the exit it had when the commit began
test.each(['chromium', 'webkit'])('a removal plays the exit its commit began with, whichever write comes first, on %s', async engine => {
  const app = await exitApp(engine, `keyframes leave
  to opacity=0

component App
  state calm = true
  state shown = true
  action calmFirst
    calm = not calm
    shown = false
  action dropFirst
    shown = false
    calm = not calm
  action show
    shown = true
  view
    column
      button "Calm first" press=calmFirst testId="calm-first"
      button "Drop first" press=dropFirst testId="drop-first"
      button "Show" press=show testId="show"
      if shown
        text "Card" testId="card" -exact-exit-animation=(calm ? "none" : "leave 2000ms linear both")
`);
  try {
    const leaving = async button => {
      const { ghosts } = await app.drop(button);
      await app.page.locator('[data-testid="show"]').click();
      await app.page.locator('[data-testid="card"]').waitFor();
      return ghosts;
    };
    // Each action flips `calm`, so the removals alternate between starting calm and starting with the exit
    expect([await leaving('calm-first'), await leaving('calm-first'), await leaving('drop-first'), await leaving('drop-first')]).toEqual([0, 1, 0, 1]);
  } finally { await app.close(); }
}, 60000);

test('an exit ghost does not run its resize action when its fractional box is rounded', async () => {
  const app = await exitApp('chromium', `keyframes leave
  to opacity=0

component App
  state shown = true
  state calls = 0
  state seen = "none"
  action hide
    shown = false
  action fit(w: number, h: number)
    calls = calls + 1
    seen = \`\${w}x\${h}\`
  view
    column
      button "hide" press=hide testId="hide"
      text \`\${calls} \${seen} \${shown}\` testId="seen"
      if shown
        box testId="panel" width=200.4 height=80.4 resize=fit -exact-exit-animation="leave 600ms linear both"
`);
  try {
    const { page } = app;
    await page.waitForFunction(() => document.querySelector('[data-testid="seen"]')?.textContent.startsWith('1 '));
    const result = await page.evaluate(async () => {
      const panel = document.querySelector('[data-testid="panel"]');
      const seen = document.querySelector('[data-testid="seen"]');
      const before = seen.textContent;
      // Wait for the browser to report the ghost's changed box before checking that no action ran.
      const resized = new Promise(resolve => {
        const observer = new ResizeObserver(entries => {
          const rect = entries.find(entry => entry.target === panel)?.contentRect;
          if (panel.hasAttribute('data-exiting') && rect?.width === 200 && rect.height === 80) {
            observer.disconnect();
            resolve();
          }
        });
        observer.observe(panel);
      });
      const exiting = new Promise(resolve => {
        const observer = new MutationObserver(() => {
          if (!panel.hasAttribute('data-exiting')) return;
          observer.disconnect();
          for (const animation of panel.getAnimations()) animation.pause();
          resolve();
        });
        observer.observe(panel, { attributes: true, attributeFilter: ['data-exiting'] });
      });
      document.querySelector('[data-testid="hide"]').click();
      await exiting;
      await resized;
      await new Promise(requestAnimationFrame);
      const after = seen.textContent, connected = panel.isConnected;
      await Promise.all(panel.getAnimations().map(animation => (animation.finish(), animation.finished)));
      return { before, after, connected, removed: !panel.isConnected };
    });
    expect(result.before).toMatch(/^1 200\.\d+x80\.\d+ true$/);
    expect(result.connected).toBe(true);
    expect(result.removed).toBe(true);
    expect(result.after).toBe(result.before.replace(' true', ' false'));
  } finally { await app.close(); }
}, 30000);

/** `source` compiled to the JS target and opened in `engine` with presence loaded. `drop(button)` presses `button`,
 * reports the exit animations that start, then finishes them; `close()` removes the page, server and app. */
async function exitApp(engine, source, pageOptions) {
  const root = resolve(import.meta.dir, '../..'), target = resolve(root, 'target');
  const dir = mkdtempSync(resolve(target, 'exit-animation-'));
  const style = withLook(readFileSync(resolve(root, 'host/web/index.html'), 'utf8')).match(/<style>([\s\S]*?)<\/style>/)[1];
  let server, browser;
  const close = async () => { await browser?.close(); server?.stop(true); rmSync(dir, { recursive: true, force: true }); };
  try {
    writeFileSync(resolve(dir, 'app.contract'), source);
    const compiled = spawnSync(resolve(target, 'debug/exact-web-js'), ['js', resolve(dir, 'app.contract'), '-o', dir], { encoding: 'utf8' });
    expect(compiled.status, compiled.stderr).toBe(0);
    server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(request) {
      const path = new URL(request.url).pathname.slice(1);
      if (!path) { return new Response(`<style>${style}</style><link rel="stylesheet" href="/app.css"><div id="exact-root"></div><script type="module">import app from '/app.js';app();</script>`, { headers: { 'content-type': 'text/html' } }); }
      const bases = path.startsWith('web/') ? [resolve(root, 'host')] : [dir, import.meta.dir, resolve(root, 'host/web')];
      const file = bases.map(base => resolve(base, path)).find(existsSync);
      return file ? new Response(Bun.file(file)) : new Response(null, { status: 404 });
    } });
    browser = await suiteBrowser(engine);
    const page = await browser.newPage(pageOptions);
    await page.goto(server.url.href);
    await page.evaluate(async () => { const { pieces } = await import('/rt.js'); await pieces(); });
    await page.waitForFunction(() => globalThis.exact?.presenceLive);
    const drop = button => page.evaluate(async button => {
      document.querySelector(`[data-testid="${button}"]`).click();
      await new Promise(resolve => setTimeout(resolve));
      const ghosts = [...document.querySelectorAll('[data-exiting]')];
      const animations = ghosts.flatMap(node => node.getAnimations({ subtree: true }));
      await Promise.all(animations.map(async animation => { animation.pause(); await animation.ready; animation.currentTime = 1000; }));
      const scales = ghosts.map(ghost => parseFloat(getComputedStyle(ghost).scale));
      const result = { ghosts: ghosts.length, scales, durations: animations.map(animation => animation.effect.getTiming().duration) };
      await Promise.all(animations.map(animation => (animation.finish(), animation.finished)));
      return { ...result, remaining: document.querySelectorAll('[data-exiting]').length };
    }, button);
    return { page, drop, close };
  } catch (e) {
    await close();
    throw e;
  }
}


test('text selection scopes release removed nodes and the document listener has one owner', async () => {
  const root = resolve(import.meta.dir, '../..');
  const server = Bun.serve({ hostname: '127.0.0.1', port: 0, fetch(request) {
    const path = new URL(request.url).pathname.slice(1);
    if (!path) return new Response('<body><div id="exact-root"></div>', { headers: { 'content-type': 'text/html' } });
    if (path === 'names.js') return new Response('export const sourceTypes={};', { headers: { 'content-type': 'text/javascript' } });
    const bases = path.startsWith('web/') ? [resolve(root, 'host')] : [resolve(root, 'host/web-js'), resolve(root, 'host/web')];
    const file = bases.map(base => resolve(base, path)).find(existsSync);
    return file ? new Response(Bun.file(file), { headers: { 'content-type': 'text/javascript' } }) : new Response('', { status: 404 });
  } });
  let browser;
  try {
    browser = await suiteBrowser(); const page = await browser.newPage(); await page.goto(server.url.href);
    const result = await page.evaluate(async () => {
      globalThis.__exactRender = true;
      const listeners = new Set(), add = document.addEventListener.bind(document), remove = document.removeEventListener.bind(document);
      document.addEventListener = (name, fn, ...rest) => { if (name === 'selectionchange') listeners.add(fn); add(name, fn, ...rest); };
      document.removeEventListener = (name, fn, ...rest) => { if (name === 'selectionchange') listeners.delete(fn); remove(name, fn, ...rest); };
      const rt = await import('/rt.js'), nav = await import('/web/navigation/selection.js');
      const root = document.getElementById('exact-root');
      for (let i = 0; i < 100; i++) {
        const node = document.createElement('p'); node.textContent = 'row ' + i; root.append(node);
        const scope = rt.scope(() => rt.onSelectionChange(node, 'selectionchange', () => {}, () => {}));
        node.remove(); rt.end(scope);
      }
      const retired = nav.selectedTexts.size, idle = listeners.size;
      const node = document.createElement('p'); node.textContent = 'hello'; root.append(node);
      let heard = 0;
      const old = nav.onSelection(node, () => { heard += 100; }), stop = nav.onSelection(node, () => heard++); old();
      const range = document.createRange(); range.selectNodeContents(node); getSelection().removeAllRanges(); getSelection().addRange(range);
      document.dispatchEvent(new Event('selectionchange'));
      const active = listeners.size; stop(); stop(); node.remove();
      return { retired, idle, heard, active, remaining: nav.selectedTexts.size, listeners: listeners.size };
    });
    expect(result).toEqual({ retired: 0, idle: 0, heard: 1, active: 1, remaining: 0, listeners: 0 });
  } finally { await browser?.close(); server.stop(true); }
}, 30000);
