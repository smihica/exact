import { expect, test } from 'bun:test';
import { hostOf } from './app-cli.mjs';
import { main } from './agent.mjs';

// The host before `test`'s files and `agent`'s ops is optional: a word is taken as the host only when it names one, so
// `agent "tap like" "state liked"` keeps both ops and `test app.test.contract` runs that file, both on the web.
test('the first word is the host only when it names one', () => {
  for (const [verb, words, host, left] of [
    ['agent', ['tap like', 'state liked'], 'web', ['tap like', 'state liked']],
    ['agent', ['ios', 'tree'], 'ios', ['tree']],
    ['test', ['app.test.contract'], 'web', ['app.test.contract']],
    ['test', ['web', 'app.test.contract'], 'web', ['app.test.contract']],
    ['verify', ['--strict'], 'web', ['--strict']],
    ['build', ['ios'], 'web', ['ios']],
  ]) {
    const rest = [...words];
    expect([hostOf(verb, rest), rest]).toEqual([host, left]);
  }
});

// The driver carries a drive only on a host it knows; any other word fell to the web and was recorded as that host's.
test('the driver refuses a host it does not carry', async () => {
  const said = [];
  const error = console.error;
  console.error = (line) => said.push(String(line));
  try {
    for (const host of ['iOS', 'tvos', 'lniux']) expect(await main([host, 'tree'])).toBe(2);
  } finally { console.error = error; }
  expect(said.every((line) => line.startsWith('agent: no host'))).toBe(true);
});
