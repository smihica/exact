// LLP 1057.003 D5 on the JS target: the timelines fixture (host/web/tests/fixtures/timelines.contract) built by this
// target runs every scroll and view timeline as the browser's own, and each consumer shows what the browser recorded
// for the wasm target's build of the same app (browser-timelines.json, `parity.mjs --timelines`), which the engine is
// held to as well. Without the fixture's drag timeline the plan has none, so every name is CSS's own row and no motion
// piece is needed; the recording itself is of the fixture whole, where the motion piece hands each name that finds a
// scroll or view timeline to the browser (motion-glue.js `followTimelines`). A browser with no scroll-driven animations
// (Firefox's) shows each consumer's own values. Build exact-web-js once before running this test (docs/testing.md).
import { afterAll, expect } from 'bun:test';
import { EXE } from '../../scripts/paths.mjs';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { suiteBrowser } from '../../scripts/agent-launch.mjs';
import { withLook } from '../web/look.mjs';
import { engineTests } from '../../scripts/coverage.mjs';

const engineTest = await engineTests(['chromium', 'firefox']);

const root = resolve(import.meta.dir, '../..'), fixtures = resolve(root, 'host/web/tests/fixtures');
const target = resolve(root, 'target');
const steps = JSON.parse(readFileSync(resolve(fixtures, 'timelines-steps.json'), 'utf8'));
const recorded = JSON.parse(readFileSync(resolve(fixtures, 'browser-timelines.json'), 'utf8')).samples;
// The fixture without its drag timeline: a plan with none.
const noDrag = readFileSync(resolve(fixtures, 'timelines.contract'), 'utf8').replace(/^.*-exact-drag-timeline.*\n/m, '');
const browsers = new Set();
afterAll(async () => { for (const b of browsers) await b.close(); });

/** `source` built by this target and opened in `engine`; `close` ends it. */
async function open(source, engine, ready) {
  mkdirSync(target, { recursive: true });
  const dir = mkdtempSync(resolve(target, 'scroll-timelines-'));
  const style = withLook(readFileSync(resolve(root, 'host/web/index.html'), 'utf8')).match(/<style>([\s\S]*?)<\/style>/)[1];
  writeFileSync(resolve(dir, 'app.contract'), source);
  const compiled = spawnSync(resolve(target, `debug/exact-web-js${EXE}`), ['js', resolve(dir, 'app.contract'), '-o', dir], { encoding: 'utf8' });
  expect(compiled.status, compiled.stderr).toBe(0);
  const server = Bun.serve({ port: 0, hostname: '127.0.0.1', fetch(request) {
    const path = new URL(request.url).pathname.slice(1);
    if (!path) return new Response(`<style>${style}</style><link rel="stylesheet" href="/app.css">
      <div id="exact-root"></div><script type="module">import app from '/app.js';app();</script>`,
    { headers: { 'content-type': 'text/html' } });
    const bases = path.startsWith('web/') ? [resolve(root, 'host')] : [dir, import.meta.dir, resolve(root, 'host/web')];
    const file = bases.map(base => resolve(base, path)).find(existsSync);
    return file ? new Response(Bun.file(file)) : new Response('Not found', { status: 404 });
  } });
  const browser = await suiteBrowser(engine);
  browsers.add(browser);
  const context = await browser.newContext({ viewport: { width: 420, height: 900 } });
  const page = await context.newPage();
  await page.goto(server.url.href);
  await page.waitForFunction((id) => document.querySelector(`[data-testid="${id}"]`), ready);
  return { page, close: async () => { await context.close(); server.stop(true); rmSync(dir, { recursive: true, force: true }); } };
}

/** Scroll `scroller` to `at` on `axis`, then each of `read`'s computed opacity once two frames have run. */
const sample = (page, scroller, axis, at, read) => page.evaluate(async ({ scroller, axis, at, read }) => {
  document.querySelector(`[data-testid="${scroller}"]`)[axis === 'x' ? 'scrollLeft' : 'scrollTop'] = at;
  await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  return Object.fromEntries(read.map((id) => [id, +getComputedStyle(document.querySelector(`[data-testid="${id}"]`)).opacity]));
}, { scroller, axis, at, read });

engineTest.chromium('scroll and view timelines in a plan with no drag timeline show what the browser recorded', async () => {
  expect(noDrag).not.toContain('-exact-drag-timeline');
  const { page, close } = await open(noDrag, 'chromium', 'vsub');
  try {
    const wrong = [];
    for (const step of steps) {
      for (const at of step.at) {
        const got = await sample(page, step.scroller, step.axis, at, step.read);
        const want = recorded.find((s) => s.scroller === step.scroller && s.at === at)?.values ?? {};
        for (const id of step.read) {
          if (!(Math.abs(got[id] - want[id]) <= 1e-3)) wrong.push(`${step.scroller}@${at} ${id}: ${got[id]}, recorded ${want[id]}`);
        }
      }
    }
    expect(wrong).toEqual([]);
  } finally { await close(); }
}, 120000);

// A bound `animation` whose shorthand the page writes again resets the timeline and the range, which are written
// again after it, from none and from another animation alike: the consumer follows its scroller, not the clock.
const DYNAMIC = `keyframes fade
  from opacity=0
  to opacity=1
component App
  state on = false
  state slow = false
  action toggle
    on = not on
  action slower
    slow = not slow
  view
    column
      button "Toggle" press=toggle testId="toggle"
      button "Slower" press=slower testId="slower"
      column timeline-scope="--t"
        row testId="s" width=200 height=40 overflow-x="auto" overflow-y="hidden" scroll-timeline="--t x"
          view width=800 height=40 flex-shrink=0
        view testId="c" width=10 height=4 opacity=0.25 animation=(on ? (slow ? "fade 2s linear both" : "fade 1s linear both") : "none") animation-timeline="--t" animation-range="0px 300px"
`;

engineTest.chromium('a bound animation under a literal timeline and range follows its scroller once the page writes it', async () => {
  const { page, close } = await open(DYNAMIC, 'chromium', 'c');
  try {
    expect(await sample(page, 's', 'x', 150, ['c'])).toEqual({ c: 0.25 });
    await page.click('[data-testid="toggle"]');
    expect(await page.evaluate(() => getComputedStyle(document.querySelector('[data-testid="c"]')).animationTimeline)).toBe('--t');
    const at = (x) => sample(page, 's', 'x', x, ['c']).then((v) => v.c);
    expect(await at(150)).toBeCloseTo(0.5, 3);
    expect(await at(75)).toBeCloseTo(0.25, 3);
    expect(await at(300)).toBeCloseTo(1, 3);
    // Active to active: another duration, the same timeline and range.
    await page.click('[data-testid="slower"]');
    const shown = () => page.evaluate(() => {
      const s = getComputedStyle(document.querySelector('[data-testid="c"]'));
      return [s.animationDuration, s.animationTimeline, s.animationRangeStart, s.animationRangeEnd];
    });
    expect(await shown()).toEqual(['2s', '--t', '0px', '300px']);
    expect(await at(150)).toBeCloseTo(0.5, 3);
    expect(await at(30)).toBeCloseTo(0.1, 3);
  } finally { await close(); }
}, 120000);

const NONE_PAUSED = `keyframes fade
  from opacity=0.2
  to opacity=0.8
component App
  state active = true
  state timeline = "none"
  action toggle
    active = not active
  action setTimeline(t: string)
    timeline = t
  view
    column
      button "Toggle" press=toggle testId="toggle"
      button "Scroll" press=setTimeline("scroll()") testId="tl-scroll"
      button "None" press=setTimeline("none") testId="tl-none"
      column testId="scroller" width=100 height=100 overflow-y="auto"
        view testId="static" width=10 height=4 opacity=1 animation="fade 1s linear both paused" animation-timeline="none"
        view testId="bound" width=10 height=4 opacity=1 animation=(active ? "fade 1s linear both paused" : "none") animation-timeline=timeline
        box height=400 flex-shrink=0
`;

engineTest.chromium('animation-timeline: none preserves authored animation-play-state: paused on static and bound animations', async () => {
  const { page, close } = await open(NONE_PAUSED, 'chromium', 'bound');
  try {
    const read = () => page.evaluate(() => ['static', 'bound'].map((id) => {
      const s = getComputedStyle(document.querySelector(`[data-testid="${id}"]`));
      return [s.animationTimeline, s.animationPlayState, +s.opacity];
    }));
    expect(await read()).toEqual([['none', 'paused', 0.2], ['none', 'paused', 0.2]]);
    await page.click('[data-testid="toggle"]');
    await page.click('[data-testid="toggle"]');
    expect(await read()).toEqual([['none', 'paused', 0.2], ['none', 'paused', 0.2]]);
    // Switching `none -> scroll() -> none` over a live scroll range must keep `paused`
    // throughout (never advancing on the `scroll()` leg or losing `paused` on return).
    await page.click('[data-testid="tl-scroll"]');
    expect(await sample(page, 'scroller', 'y', 150, ['bound'])).toEqual({ bound: 0.2 });
    expect(await read()).toEqual([['none', 'paused', 0.2], ['scroll()', 'paused', 0.2]]);
    await page.click('[data-testid="tl-none"]');
    expect(await read()).toEqual([['none', 'paused', 0.2], ['none', 'paused', 0.2]]);
  } finally { await close(); }
}, 120000);

const DRAG_GLUE_AND_EXIT = `keyframes fade
  from opacity=0.2
  to opacity=0.8
keyframes fade-paused
  from opacity=0.1
  to opacity=0.9
keyframes shift
  from translate="20px 0px"
  to translate="120px 0px"
keyframes leave
  from opacity=1
  to opacity=0
component App
  state hasScroll = true
  state hasDynScroll = false
  state dynTimeline = "--dyn"
  state dynPaused = false
  state dragX = 0
  state dragAnimPaused = true
  state dragDur = 1
  state releaseX = 0
  state releasePaused = false
  state noneCase = " NONE "
  state showExit = true
  state showSameNameExit = true
  action dropScroll
    hasScroll = false
  action enableDynScroll
    hasDynScroll = true
  action pauseDynScroll
    dynPaused = true
  action clearDynTimeline
    dynTimeline = "auto"
  action moveDrag
    dragX = 80
  action setDragHalf
    dragX = 50
    dragAnimPaused = false
  action pauseDragAtHalf
    dragAnimPaused = true
  action retimeDrag
    dragDur = 2
  action startRelease
    releaseX = 100
  action pauseRelease
    releasePaused = true
  action hideExit
    showExit = false
  action hideSameNameExit
    showSameNameExit = false
  view
    column timeline-scope="--drag, --dyn, --rel"
      button "DropScroll" press=dropScroll testId="drop-scroll"
      button "EnableDyn" press=enableDynScroll testId="enable-dyn"
      button "PauseDyn" press=pauseDynScroll testId="pause-dyn"
      button "ClearDyn" press=clearDynTimeline testId="clear-dyn"
      button "MoveDrag" press=moveDrag testId="move-drag"
      button "SetDragHalf" press=setDragHalf testId="set-drag-half"
      button "PauseDragHalf" press=pauseDragAtHalf testId="pause-drag-half"
      button "RetimeDrag" press=retimeDrag testId="retime-drag"
      button "StartRelease" press=startRelease testId="start-release"
      button "PauseRelease" press=pauseRelease testId="pause-release"
      button "HideExit" press=hideExit testId="hide-exit"
      button "HideSameName" press=hideSameNameExit testId="hide-same-name"
      column testId="scroller" width=100 height=100 overflow-y="auto" scroll-timeline-name=(hasScroll ? "--sc" : "none")
        view testId="paused-css" width=10 height=4 opacity=1 animation="fade 1s linear both paused" animation-timeline="--sc"
        box height=400 flex-shrink=0
      column testId="dyn-scroller" width=100 height=100 overflow-y="auto" scroll-timeline-name=(hasDynScroll ? "--dyn" : "none")
        box height=400 flex-shrink=0
      view testId="dyn-consumer" width=10 height=4 opacity=1 animation=(dynPaused ? "fade 1s linear both paused" : "fade 1s linear both") animation-timeline=dynTimeline
      view testId="dragger" width=20 height=20 -exact-drag-timeline="--drag x" translate=\`\${dragX}px 0px\`
      view testId="paused-drag" width=10 height=4 opacity=1 animation=\`fade \${dragDur}s linear both \${dragAnimPaused ? "paused" : "running"}\` animation-timeline="--drag" animation-range="0px 100px"
      view testId="hyphen-drag" width=10 height=4 opacity=1 animation="fade-paused 1s linear both" animation-timeline="--drag" animation-range="0px 100px"
      view testId="releaser" width=20 height=20 -exact-drag-timeline="--rel x" transition="translate 0.6s linear" translate=\`\${releaseX}px 0px\`
      view testId="release-consumer" width=10 height=4 opacity=1 animation=(releasePaused ? "fade 1s linear both paused" : "fade 1s linear both") animation-timeline="--rel" animation-range="0px 100px"
      view testId="upper-none" width=10 height=4 opacity=1 animation="fade 1s linear both" animation-timeline=noneCase
      if showExit
        view testId="exiter" width=20 height=20 opacity=1 animation="shift 1s linear both" animation-timeline="none" -exact-exit-animation="leave 0.3s linear both"
      if showSameNameExit
        view testId="same-name-exiter" width=20 height=20 opacity=1 animation="fade 1s linear both" animation-timeline="none" -exact-exit-animation="fade 0.2s linear both"
`;

engineTest.chromium('drag-timeline motion glue preserves authored pauses, detaches stale CSS timelines, and keeps ordinary timelines across exits', async () => {
  mkdirSync(target, { recursive: true });
  const dir = mkdtempSync(resolve(target, 'scroll-timelines-drag-'));
  writeFileSync(resolve(dir, 'app.json'), JSON.stringify({ name: 'Drag Timelines', app: { id: 'test.drag-timelines', name: 'Drag Timelines' }, host: { web: {} } }));
  writeFileSync(resolve(dir, 'app.contract'), DRAG_GLUE_AND_EXIT);
  const dist = resolve(dir, 'dist');
  const built = spawnSync(process.execPath, ['host/web-js/build.mjs', 'drag-timelines', '--out', dist, '--render', 'none'], {
    cwd: root, env: { ...process.env, EXACT_APP_DIR: dir }, encoding: 'utf8',
  });
  expect(built.status, built.stdout + built.stderr).toBe(0);
  const server = Bun.serve({ port: 0, hostname: '127.0.0.1', async fetch(request) {
    const path = new URL(request.url).pathname;
    const file = Bun.file(resolve(dist, path === '/' || !path.includes('.') ? 'index.html' : path.slice(1)));
    return await file.exists() ? new Response(file) : new Response('Not found', { status: 404 });
  } });
  const browser = await suiteBrowser('chromium');
  browsers.add(browser);
  const context = await browser.newContext({ viewport: { width: 420, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  try {
    await page.goto(server.url.href);
    await page.waitForFunction(() => document.querySelector('[data-testid="exiter"]'));
    // Wait for motion-glue.js to load and run its initial followTimelines() pass.
    await page.waitForFunction(() => getComputedStyle(document.querySelector('[data-testid="paused-css"]')).animationTimeline === '--sc');
    expect(await sample(page, 'scroller', 'y', 150, ['paused-css'])).toEqual({ 'paused-css': 0.2 });
    const cssState = await page.evaluate(() => {
      const s = getComputedStyle(document.querySelector('[data-testid="paused-css"]'));
      return [s.animationTimeline, s.animationPlayState, +s.opacity];
    });
    expect(cssState).toEqual(['--sc', 'paused', 0.2]);

    // Case-insensitive/padded `" NONE "` normalizes `--exact-animation-timeline` to `"none"`.
    expect(await page.evaluate(() => document.querySelector('[data-testid="upper-none"]').style.getPropertyValue('--exact-animation-timeline').trim())).toBe('none');

    // Keyframe name `fade-paused` must not be mistaken for authored `paused`: it follows dragX -> 80%.
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="paused-drag"]')).opacity)).toBeCloseTo(0.2, 3);
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="hyphen-drag"]')).opacity)).toBeCloseTo(0.1, 3);
    await page.click('[data-testid="move-drag"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="paused-drag"]')).opacity)).toBeCloseTo(0.2, 3);
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="hyphen-drag"]')).opacity)).toBeCloseTo(0.74, 2);

    // Retiming a paused drag consumer at 50% (opacity 0.5) after moving dragX to 80%
    // proves normalized progress stays at 0.5 (neither 0.35 from unscaled time nor 0.68 from re-seeking).
    await page.click('[data-testid="set-drag-half"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="paused-drag"]')).opacity)).toBeCloseTo(0.5, 2);
    await page.click('[data-testid="pause-drag-half"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    await page.click('[data-testid="move-drag"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    await page.click('[data-testid="retime-drag"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="paused-drag"]')).opacity)).toBeCloseTo(0.5, 2);

    // Pausing during an active drag-release transition freezes at the mid-release sample (strictly between 0.3 and 0.78).
    await page.click('[data-testid="start-release"]');
    await page.waitForFunction(() => +getComputedStyle(document.querySelector('[data-testid="release-consumer"]')).opacity > 0.32);
    await page.click('[data-testid="pause-release"]');
    const pausedRelease = await page.evaluate(async () => {
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      return +getComputedStyle(document.querySelector('[data-testid="release-consumer"]')).opacity;
    });
    expect(pausedRelease).toBeGreaterThan(0.3);
    expect(pausedRelease).toBeLessThan(0.78);
    await page.evaluate(() => new Promise(r => setTimeout(r, 150)));
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="release-consumer"]')).opacity)).toBeCloseTo(pausedRelease, 2);

    // Inactive named CSS consumer (`--dyn` has no source yet -> shows underlying opacity 1),
    // becomes live CSS scroll consumer when `--dyn` appears, honors authored `paused`,
    // and clears its inline `animation-timeline` when switched back to `auto`.
    expect(await page.evaluate(() => +getComputedStyle(document.querySelector('[data-testid="dyn-consumer"]')).opacity)).toBeCloseTo(1, 3);
    await page.click('[data-testid="enable-dyn"]');
    await page.waitForFunction(() => getComputedStyle(document.querySelector('[data-testid="dyn-consumer"]')).animationTimeline === '--dyn');
    expect((await sample(page, 'dyn-scroller', 'y', 150, ['dyn-consumer']))['dyn-consumer']).toBeCloseTo(0.5, 2);
    await page.click('[data-testid="pause-dyn"]');
    await page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
    expect((await sample(page, 'dyn-scroller', 'y', 300, ['dyn-consumer']))['dyn-consumer']).toBeCloseTo(0.5, 2);
    await page.click('[data-testid="clear-dyn"]');
    await page.waitForFunction(() => document.querySelector('[data-testid="dyn-consumer"]').style.getPropertyValue('animation-timeline') === '');

    // Dropping the named CSS scroll timeline without restyling the consumer detaches the stale `--sc` timeline.
    await page.click('[data-testid="drop-scroll"]');
    await page.waitForFunction(() => document.querySelector('[data-testid="paused-css"]').style.getPropertyValue('animation-timeline') === 'none');

    // Exiting a class-styled element with `animation-timeline: none` keeps its ordinary animation on `none`
    // while playing `-exact-exit-animation` on `auto`, then removes the ghost.
    await page.click('[data-testid="hide-exit"]');
    const exitMid = await page.evaluate(async () => {
      await new Promise(r => requestAnimationFrame(r));
      const ghost = document.querySelector('[data-testid="exiter"][data-exiting]');
      if (!ghost) return null;
      const cs = getComputedStyle(ghost);
      return { names: cs.animationName, timelines: cs.animationTimeline, translate: cs.translate };
    });
    expect(exitMid).not.toBeNull();
    expect(exitMid.names).toContain('shift');
    expect(exitMid.names).toContain('leave');
    expect(exitMid.timelines).toBe('none, auto');
    expect(exitMid.translate).toBe('20px');
    await page.waitForFunction(() => !document.querySelector('[data-testid="exiter"]'));

    // Reusing the same keyframe name (`fade`) for ordinary `none` and `-exact-exit-animation`
    // must wait on the exit animation rather than stranding the ghost on the detached `none` animation.
    await page.click('[data-testid="hide-same-name"]');
    await page.waitForFunction(() => !document.querySelector('[data-testid="same-name-exiter"]'));

    // `timelineReleasers` stays registered across motion controller `reset()` (WASM reload safety).
    expect(await page.evaluate(() => globalThis.exact?.timelineReleasers?.size ?? 0)).toBeGreaterThan(0);
    expect(errors).toEqual([]);
  } finally {
    await context.close();
    server.stop(true);
    rmSync(dir, { recursive: true, force: true });
  }
}, 120000);

engineTest.firefox('a browser with no scroll-driven animations shows every consumer its own values', async () => {
  for (const [source, ready, groups] of [
    [noDrag, 'vsub', steps],
    [DYNAMIC, 'c', [{ scroller: 's', axis: 'x', at: [0, 150, 300], read: ['c'], toggle: true }]],
  ]) {
    const { page, close } = await open(source, 'firefox', ready);
    try {
      expect(await page.evaluate(() => CSS.supports('animation-timeline', 'scroll()'))).toBe(false);
      const wrong = [];
      for (const step of groups) {
        if (step.toggle) await page.click('[data-testid="toggle"]');
        for (const at of step.at) {
          const got = await sample(page, step.scroller, step.axis, at, step.read);
          for (const id of step.read) if (got[id] !== 0.25) wrong.push(`${step.scroller}@${at} ${id}: ${got[id]}`);
        }
      }
      expect(wrong).toEqual([]);
    } finally { await close(); }
  }
}, 240000);
