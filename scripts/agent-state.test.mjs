// Authored state comparisons and their browser diagnostics.
import { expect, test } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runTests, stateHolds } from './agent-test.mjs';
import { serveBuildTree } from '../host/web/serve.mjs';
import { engineTests } from './coverage.mjs';

const engineTest = await engineTests();

test('script state comparisons use inclusive numeric bounds and typed literal membership', () => {
  const holds=(got,comparison,value)=>stateHolds(got,{comparison,value});
  expect(holds(-2,'at-least',-2)).toBe(true); expect(holds(2,'at-most',2)).toBe(true);
  expect(holds(-3,'at-least',-2)).toBe(false); expect(holds(3,'at-most',2)).toBe(false);
  for (const got of ['2',null,false,[],{}]) { expect(holds(got,'at-least',1)).toBe(false); expect(holds(got,'at-most',3)).toBe(false); }
  for (const got of [-2,'ready',false,null,[]]) expect(holds(got,'in',[-2,'ready',false,null,[]])).toBe(true);
  expect(holds('2','in',[2])).toBe(false); expect(holds(2,'in',['2'])).toBe(false);
  expect(holds([],'equal',[])).toBe(true);
});

test('script state equality compares lists and records structurally', () => {
  const holds = (got, value) => stateHolds(got, { comparison: 'equal', value });
  expect(holds(['a', 'b'], ['a', 'b'])).toBe(true);
  expect(holds(['a', 'b'], ['a', 'c'])).toBe(false);
  expect(holds(['a', 'b'], ['a'])).toBe(false);
  expect(holds(['a', 'b'], ['b', 'a'])).toBe(false);
  expect(holds([[1, -2], [], [3]], [[1, -2], [], [3]])).toBe(true);
  expect(holds([[1, -2], [], [3]], [[1, 2], [], [3]])).toBe(false);
  expect(holds([], [])).toBe(true);
  expect(holds([], [1])).toBe(false);
  expect(holds([1], ['1'])).toBe(false);
  expect(holds({ id: 'a', position: { x: 1, y: -2 } }, { position: { y: -2, x: 1 }, id: 'a' })).toBe(true);
  expect(holds({ id: 'a' }, { key: 'a' })).toBe(false);
  expect(holds({}, [])).toBe(false);
  expect(holds([NaN], [null])).toBe(false);
  expect(holds([NaN], [NaN])).toBe(false);
  expect(holds([-0], [0])).toBe(true);
});

engineTest.chromium('authored state literals compare whole values and report mismatches', async () => {
  const root = fileURLToPath(new URL('..', import.meta.url));
  const dir = mkdtempSync(join(root, 'target/state-literals-'));
  const app = join(dir, 'app.contract'), file = join(dir, 'app.test.contract');
  const plan = join(dir, 'app.plan'), dist = join(dir, 'dist');
  const server = createServer((req, res) => serveBuildTree(dist, req, res));
  try {
    writeFileSync(app, `shape Point
  x: number
  y: number
component App
  state picks = ["a", "b"]
  state grid = [[1, -2], [], [3]]
  state empty: number[] = []
  state picked = some(["a", "b"])
  state point = Point(x=1, y=-2)
  view
    text "Hi"
`);
    writeFileSync(file, `test "equal literals"
  expect state picks == ["a", "b"]
  expect state grid == [[1, -2], [], [3,],]
  expect state empty == []
  expect state picked == some(["a", "b"])
  expect state point == Point(y=-2, x=1)
test "unequal literals"
  expect state picks == ["a", "c"]
  expect state picks == ["a"]
  expect state grid == [[1, 2], [], [3]]
  expect state empty == [1]
  expect state picked == none
  expect state point == Point(x=2, y=-2)
`);
    const compiler = join(process.env.CARGO_TARGET_DIR ?? join(root, 'target'), 'debug', 'contract');
    const compile = spawnSync(compiler, ['build', app, '-o', plan], { cwd: root, encoding: 'utf8' });
    expect(compile.status, compile.stderr).toBe(0);
    const build = spawnSync(process.execPath, ['host/web-js/build.mjs', 'ui-gallery', '--plan', plan, '--out', dist, '--render', 'none'], { cwd: root, encoding: 'utf8' });
    expect(build.status, build.stderr).toBe(0);
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const previous = process.env.EXACT_WEB_DIST;
    let r;
    try {
      process.env.EXACT_WEB_DIST = dist;
      r = await runTests({ host: 'web', file, app: 'ui-gallery', url: `http://127.0.0.1:${server.address().port}/` });
    } finally {
      if (previous === undefined) delete process.env.EXACT_WEB_DIST; else process.env.EXACT_WEB_DIST = previous;
    }
    expect([r.passed, r.failed]).toEqual([1, 1]);
    expect(r.results[0].failures).toEqual([]);
    const messages = r.results[1].failures.map(f => f.slice(f.indexOf(': ', f.indexOf(':') + 1) + 2));
    expect(messages).toEqual([
      'picks is ["a","b"], expected ["a","c"]',
      'picks is ["a","b"], expected ["a"]',
      'grid is [[1,-2],[],[3]], expected [[1,2],[],[3]]',
      'empty is [], expected [1]',
      'picked is ["a","b"], expected null',
      'point is {"x":1,"y":-2}, expected {"x":2,"y":-2}',
    ]);
  } finally { server.close(); rmSync(dir, { recursive: true, force: true }); }
}, 180_000);
