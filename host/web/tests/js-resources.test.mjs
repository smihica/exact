// A resource's answers on the JS target, over the runtime's own modules in Bun (js-runtime-harness.mjs).
import { expect, test } from 'bun:test';
import { runtimeHarness } from './js-runtime-harness.mjs';

const { freshRuntime } = runtimeHarness();

// A Rust source's answer at once outside its shape fails the resource with `shape`, as a TypeScript source's reply
// does: the commit stands and the last answer shows (runner `shaped_answer`). With nothing to show, neither an
// answer nor an `else`, the shape refusal refuses the commit, as the runner's does.
test('an answer at once outside its shape fails the resource and the commit stands', async () => {
  const { res, W, data, commit, sig, act, journal } = await freshRuntime('shape-now');
  data.answer = (source, [mode]) => (source === 'shapeProbe' ? { v: mode === 'good' ? 1 : 'extra' } : null);
  const mode = sig('good', 's');
  let probe;
  commit(() => { probe = res('shapeProbe', 'shapeProbe', () => [mode()], undefined, undefined, 'n', 0); probe(); });
  act(() => W(mode, 'extra'))();
  expect([mode(), probe(), probe.f(), probe.e()?.[0]]).toEqual(['extra', 1, true, 'shape']);
  const bare = res('bareProbe', 'shapeProbe', () => ['extra'], undefined, undefined, 'n');
  expect(commit(() => bare())).toBe(false);
  expect(journal.at(-1)).toContain('bareProbe: the answer does not conform to its shape');
});
