import { test, expect } from 'bun:test';
import { implementation, pieces } from '../web/index.js';

test.each([
  ['ASCII', 'a', 40960, [16384, 16384, 8192]],
  ['two-byte', 'é', 9000, [16384, 1616]],
  ['three-byte', 'あ', 6000, [16383, 1617]],
  ['long three-byte', 'あ', 20000, [16383, 16383, 16383, 10851]],
  ['four-byte', '😀', 5000, [16384, 3616]],
  ['exactly 16 KB', '😀', 4096, [16384]],
  ['short', 'あ', 2, [6]],
])('%s speech text is split into nonempty pieces of at most 16 KB', (_, character, count, sizes) => {
  const text = character.repeat(count), parts = pieces(text);
  expect(parts.map(p => new TextEncoder().encode(p).length)).toEqual(sizes);
  expect(parts.join('')).toBe(text);
});

test('a speech piece never cuts a surrogate pair at the byte limit', () => {
  const prefix = 'a'.repeat(16383), parts = pieces(`${prefix}😀x`);
  expect(parts.map(p => new TextEncoder().encode(p).length)).toEqual([16383, 5]);
  expect(parts).toEqual([prefix, '😀x']);
});

test('speech pieces prefer a space only within the byte limit', () => {
  const first = 'あ'.repeat(5400), last = 'い'.repeat(600);
  expect(pieces(`${first} ${last}`)).toEqual([first, last]);
  const text = `${'あ'.repeat(5990)} ${'い'.repeat(9)}`, parts = pieces(text);
  expect(parts.map(p => new TextEncoder().encode(p).length)).toEqual([16383, 1615]);
  expect(parts.join('')).toBe(text);
});

test('speech splitting drops an empty tail but keeps an empty input for nomatch', () => {
  expect(pieces('')).toEqual(['']);
  const text = 'a'.repeat(16384), parts = pieces(`${text} \t\n`);
  expect(parts.map(p => p.length)).toEqual([16384]);
  expect(parts).toEqual([text]);
});

// The web implementation's agent path against a context of fakes (LLP 1133.012
// §9): the session clock is `clock.now`, the journal `lines`, and the store a map.
function context() {
  const clock = { now: 0 }, lines = [], files = new Map(), held = [];
  const ctx = {
    agent: true, now: () => clock.now, log: (l) => lines.push(l), changed: () => {},
    tracks: {
      lookup: () => null, subscribe: () => null,
      hold: (kind) => { if (held.includes(kind)) throw Object.assign(new Error('held'), { code: 'NotReadableError' }); held.push(kind); return () => held.splice(held.indexOf(kind), 1); },
    },
    files: {
      read: async (p) => files.get(p), write: async (p, b) => { files.set(p, b); }, remove: async (p) => { for (const k of [...files.keys()]) if (k.startsWith(p)) files.delete(k); },
      append: async (p, b) => {
        const previous = new Uint8Array(files.get(p)), bytes = new Uint8Array(previous.length + b.byteLength);
        bytes.set(previous);
        bytes.set(new Uint8Array(b), previous.length);
        files.set(p, bytes.buffer);
      },
      mkdir: async () => {}, list: async (p) => [...files.keys()].filter((k) => k.startsWith(`${p}/`)).map((k) => k.slice(p.length + 1)),
    },
  };
  return { ctx, clock, lines, files, held };
}

test('a recognition whose reply was dropped is aborted, and its hold released', async () => {
  const { ctx, lines, held } = context(), impl = implementation(ctx);
  const info = await impl.startSpeechRecognition({ microphone: true, continuous: true });
  expect(held).toEqual(['audio']);
  expect(await impl.startSpeechRecognition({ microphone: true })).toEqual({ error: 'NotReadableError' });
  impl.dispose([info.id]);
  const state = await impl.speechRecognitionState({ recognition: info.id });
  expect(state).toMatchObject({ state: 'ended', error: 'aborted' });
  expect(lines.some((l) => l.includes(`${info.id}: its reply was dropped; aborted`))).toBe(true);
  expect(held).toEqual([]);
});

test('a phrase arrives a word at a time, its final appended to the transcript', async () => {
  const { ctx, clock, files } = context(), impl = implementation(ctx);
  const info = await impl.startSpeechRecognition({ microphone: true, interimResults: true, transcript: 'app:/data/t.transcript' });
  impl.preferred({ heard: 'pick up milk' });
  // A word every 300 ms, the final 500 ms after the last.
  clock.now = 650;
  const interim = await impl.speechRecognitionState({ recognition: info.id });
  expect(interim.interim.alternatives[0].transcript).toBe('pick up');
  expect(interim.results).toEqual([]);
  clock.now = 1350;
  expect((await impl.speechRecognitionState({ recognition: info.id })).finalCount).toBe(0);
  clock.now = 1400;
  const done = await impl.speechRecognitionState({ recognition: info.id });
  expect(done.state).toBe('ended');
  expect(done.finalCount).toBe(1);
  const stopped = await impl.stopSpeechRecognition({ recognition: info.id });
  expect(stopped).toMatchObject({ finalCount: 1, files: 1, error: null });
  const line = JSON.parse(new TextDecoder().decode(files.get('app:/data/t.transcript/00000.jsonl')).trim());
  expect(line.alternatives[0].transcript).toBe('pick up milk');
});

test('processLocally where the recognizer cannot keep audio on the device fails service-not-allowed', async () => {
  const { ctx, held } = context(), impl = implementation(ctx);
  impl.preferred({ speechRecognitionStatus: { local: false } });
  const info = await impl.startSpeechRecognition({ microphone: true, processLocally: true });
  expect(info.error).toBe('service-not-allowed');
  expect(held).toEqual([]);
});

test.each([
  ['Japanese', 'あ'.repeat(6000), [16383, 1617]],
  ['trailing whitespace', `${'a'.repeat(16384)} \t\n`, [16384]],
])('%s speech pieces are numbered and written without an empty result', async (_, text, sizes) => {
  const { ctx, files } = context(), impl = implementation(ctx);
  impl.preferred({ heard: text });
  const info = await impl.startSpeechRecognition({ path: 'app:/data/t.wav', transcript: 'app:/data/t.transcript' });
  const stopped = await impl.stopSpeechRecognition({ recognition: info.id });
  expect(stopped).toMatchObject({ finalCount: sizes.length, files: 1, error: null });
  const state = await impl.speechRecognitionState({ recognition: info.id });
  expect(state.results.map(r => r.index)).toEqual(sizes.map((_, i) => i));
  const texts = state.results.map(r => r.alternatives[0].transcript);
  expect(texts.map(t => new TextEncoder().encode(t).length)).toEqual(sizes);
  expect(texts.join('')).toBe(text.trimEnd());
  const lines = new TextDecoder().decode(files.get('app:/data/t.transcript/00000.jsonl')).trim().split('\n').map(JSON.parse);
  expect(lines).toEqual(state.results);
});
