import { hostFixtureFile as readFileSync } from './glue-fixture.mjs';
// Work after a rendering opportunity (host/web/navigation.js `afterPaint`):
// two animation frames or a second, whichever is first, run once, so a page in
// a cross-origin frame scrolled out of view (the design tool's canvas) still
// loads a canvas's GPU module and its Canvas 2D piece (LLP 1122's queue item
// (3), its QUEUE follow-up; review2's 10:26 conditions). The functions are
// read from the sources and run against stand-in frames and timers.
import { expect, test } from 'bun:test';
import { glueCase, glueDeclarations } from './glue-fixture.mjs';

const navigation = readFileSync(new URL('../navigation.js', import.meta.url), 'utf8');
const glue = readFileSync(new URL('../glue.js', import.meta.url), 'utf8');
const afterPaint = glueDeclarations(navigation, ['afterPaint']);
const loadGpu = glue.match(/function loadGpuIfNeeded\(\) \{[^]*?\n\}/)[0];
const dropSurface = glue.match(/function dropSurface\(id\) \{[^{}]*\}/)[0];

test('after-paint loads reuse published exports and retry a failed load', async () => {
  const scripts = [], exact = {};
  const document = { createElement: () => ({}), head: { append: script => scripts.push(script) } };
  const source = glueDeclarations(glue, ['loadAfterPaint']).replace('import.meta.url', JSON.stringify('http://localhost/glue.js'));
  const load = new Function('document', 'globalThis', `${source}\nreturn loadAfterPaint;`)(document, { exact });
  const failed = load('./storage-environment.js', 'retrySecretWrite');
  scripts[0].onerror();
  await expect(failed).rejects.toThrow('host module loader failed: ./storage-environment.js');
  const retried = load('./storage-environment.js', 'retrySecretWrite');
  expect(scripts).toHaveLength(2);
  expect(scripts[1]).toMatchObject({ type: 'module', src: 'http://localhost/storage-environment.js' });
  exact.retrySecretWrite = () => {};
  scripts[1].onload();
  expect(await retried).toBe(exact.retrySecretWrite);
  for (let i = 0; i < 10; i++) expect(await load('./storage-environment.js', 'retrySecretWrite')).toBe(exact.retrySecretWrite);
  expect(scripts).toHaveLength(2);
});

// Stand-in frames and timers: each queued, fired when the test says.
const world = (exact = {}) => {
  const frames = [], timers = [], loads = [];
  const made = new Function('requestAnimationFrame', 'setTimeout', 'loadAfterPaint', 'exact',
    `const host = { gpuLoading: null }; const globalThis = { exact }; ${afterPaint}\n${loadGpu}\n${dropSurface}\nreturn { afterPaint, loadGpuIfNeeded, dropSurface };`)(
    fn => frames.push(fn), (fn, ms) => timers.push([fn, ms]), file => { loads.push(file); return Promise.resolve(); }, exact);
  const frame = () => { const now = frames.splice(0); for (const fn of now) fn(); };
  return { ...made, frames, timers, loads, frame, timer: () => timers.splice(0).forEach(([fn]) => fn()) };
};

test('afterPaint runs its work once: after two frames, or a second with no frames, whichever comes first', () => {
  // Frames first, then the second's timer: once (the work returns nothing; the guard is a flag, not its value).
  let runs = 0, w = world();
  w.afterPaint(() => { runs++; });
  expect(w.timers.map(([, ms]) => ms)).toEqual([1000]);
  w.frame();
  expect(runs).toBe(0);
  w.frame();
  expect(runs).toBe(1);
  w.timer();
  expect(runs).toBe(1);
  // No frames (a frame scrolled away): the second's timer, then frames that come back later, once.
  runs = 0; w = world();
  w.afterPaint(() => { runs++; });
  w.timer();
  expect(runs).toBe(1);
  w.frame(); w.frame();
  expect(runs).toBe(1);
  // Under the agent, whose clock owns time: frames only, no timer.
  runs = 0; w = world();
  w.afterPaint(() => { runs++; }, true);
  expect(w.timers).toEqual([]);
  w.frame(); w.frame();
  expect(runs).toBe(1);
});

test('a canvas surface loads its GPU module after paint, once; one destroyed before then loads none', () => {
  // A surface queued, the load scheduled after paint: the module is asked for once, frames or the second.
  const exact = { pendingSurfaces: [{ id: 7, name: 'world', values: [], generation: 0 }] };
  let w = world(exact);
  w.afterPaint(w.loadGpuIfNeeded);
  w.timer();
  expect(w.loads).toEqual(['./gpu-glue.js']);
  w.frame(); w.frame();
  expect(w.loads).toEqual(['./gpu-glue.js']);
  // Destroyed before its load ran (glue.js's `destroy` op): nothing is asked for, at the second or after frames.
  const gone = { pendingSurfaces: [{ id: 7, name: 'world', values: [], generation: 0 }, { id: 9, name: 'other', values: [], generation: 0 }] };
  w = world(gone);
  w.afterPaint(w.loadGpuIfNeeded);
  w.dropSurface(7); w.dropSurface(9);
  w.timer(); w.frame(); w.frame();
  expect([w.loads, gone.pendingSurfaces]).toEqual([[], []]);
  // One of two destroyed: the other still loads the module.
  const one = { pendingSurfaces: [{ id: 7, name: 'world', values: [], generation: 0 }, { id: 9, name: 'other', values: [], generation: 0 }] };
  w = world(one);
  w.afterPaint(w.loadGpuIfNeeded);
  w.dropSurface(7);
  w.timer();
  expect([w.loads, one.pendingSurfaces.map(s => s.id)]).toEqual([['./gpu-glue.js'], [9]]);
});

test("glue.js schedules each GPU load through afterPaint and takes a destroyed surface out of the queue; gpu-modules.js's own wait races the second too", async () => {
  // Where the loads are scheduled (a surface op and a restart), and the destroy op: through the helpers above, so a
  // revert of either site fails here (no GPU app runs in the test browser).
  const surface = glueCase(glue, 'surface'), destroy = glueCase(glue, 'destroy');
  expect(surface).toContain('afterPaint(loadGpuIfNeeded, agentMode)');
  expect(surface).not.toContain('requestAnimationFrame(');
  expect(glue).toContain('if (bytes) afterPaint(loadGpuIfNeeded, agentMode);');
  expect(destroy).toContain('dropSurface(op.id)');
  // gpu-modules.js: a later artifact (one a restart brings) loads after frames or the second, whichever is first;
  // under the agent, frames only.
  const modules = readFileSync(new URL('../gpu-modules.js', import.meta.url), 'utf8');
  const wait = modules.match(/^const afterPaint = \(\) => new Promise\(.*\);$/m)[0];
  for (const agent of [false, true]) {
    const frames = [], timers = [];
    let done = 0;
    const waited = new Function('requestAnimationFrame', 'setTimeout', 'exact', `${wait}\nreturn afterPaint();`)(fn => frames.push(fn), (fn, ms) => timers.push([fn, ms]), agent ? { agent: {} } : {})
      .then(() => { done++; });
    expect(timers.map(([, ms]) => ms)).toEqual(agent ? [] : [1000]);
    // The second alone ends the wait where there is one; frames, after it or without it, end it once.
    if (!agent) { timers[0][0](); await Promise.resolve(); await Promise.resolve(); expect(done).toBe(1); }
    frames.splice(0).forEach(fn => fn()); frames.splice(0).forEach(fn => fn());
    await waited;
    expect([agent, done]).toEqual([agent, 1]);
  }
});
