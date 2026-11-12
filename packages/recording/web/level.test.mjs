import { expect, test } from 'bun:test';
import { implementation } from './index.js';

// A meter answers every interval since its last answer. For a track that is ended, idle or muted that is silence,
// and a meter unread for hours (a hidden tab) built one entry per 50 ms and spread them into one call, which threw
// RangeError past the engine's argument limit.
test('a silent meter read after hours answers its history and the whole duration', async () => {
  let t = 0;
  const impl = implementation({ changed() {}, agent: false, now: () => t, log() {}, tracks: { subscribe: () => () => {} }, files: {} });
  const first = await impl.audioLevel({ track: 'mic' });
  expect(first.levels).toEqual([]);
  t += 10 * 60 * 60 * 1000;
  const later = await impl.audioLevel({ track: 'mic' });
  expect(later.levels.length).toBe(100);
  expect(later.levels.every((level) => level === 0)).toBe(true);
  expect(later.totalSamplesDuration).toBeCloseTo(36000, 6);
  expect(later.audioLevel).toBe(0);
});
