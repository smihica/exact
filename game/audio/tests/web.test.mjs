import { beforeAll, expect } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { chromium, suiteBrowser } from '../../../scripts/agent-launch.mjs';
import { need } from '../../../scripts/coverage.mjs';

const test = need('chrome', chromium().unavailable);
const game = resolve(import.meta.dir, '../..');
const fixture = resolve(game, 'target/audio-web-test');
beforeAll(() => {
  for (const [command, args] of [
    ['cargo', ['build', '--offline', '--locked', '-p', 'exact-game-audio', '--example', 'web_probe', '--target', 'wasm32-unknown-unknown']],
    ['wasm-bindgen', ['target/wasm32-unknown-unknown/debug/examples/web_probe.wasm', '--target', 'web', '--out-dir', fixture]],
  ]) {
    const result = spawnSync(command, args, { cwd: game, encoding: 'utf8' });
    if (result.error || result.status !== 0) throw new Error(`${command}: ${result.error ?? result.stderr}`);
  }
}, 180_000);

async function render(frames, updates, sine = false, player = false, sourceFrames = frames * 32) {
  const browser = await suiteBrowser();
  const page = await browser.newPage();
  try {
    await page.route('http://audio.test/**', route => {
      const path = new URL(route.request().url()).pathname;
      const type = path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript';
      return route.fulfill(path === '/'
        ? { contentType: 'text/html', body: '<!doctype html>' }
        : { contentType: type, body: readFileSync(resolve(fixture, path.slice(1))) });
    });
    await page.goto('http://audio.test/');
    return await page.evaluate(async ({ frames, updates, sine, player, sourceFrames }) => {
      const context = new OfflineAudioContext(2, frames, 48000);
      // Keep the production graph and automation; substitute only the device
      // clock and activation so the rendered samples are deterministic.
      let starts = 0, stops = 0;
      const rates = [], rateSchedules = [], ended = [], observations = [];
      const createSource = context.createBufferSource.bind(context);
      context.createBufferSource = () => {
        const source = createSource();
        ended.push(new Promise(resolve => source.addEventListener('ended', resolve, { once: true })));
        const start = source.start.bind(source), stop = source.stop.bind(source);
        source.start = (...args) => { starts++; return start(...args); };
        source.stop = (...args) => { stops++; return stop(...args); };
        const param = source.playbackRate;
        const schedule = { events: [], anchors: [], peak: 0 };
        rateSchedules.push(schedule);
        for (const method of ['setTargetAtTime', 'setValueAtTime', 'cancelScheduledValues']) {
          const original = param[method].bind(param);
          param[method] = (...args) => {
            if (method === 'cancelScheduledValues') {
              schedule.events = schedule.events.filter(event => event[1] < args[0]);
            } else {
              schedule.events.push(args);
              schedule.peak = Math.max(schedule.peak, schedule.events.length);
              if (method === 'setTargetAtTime') rates.push(args);
              else schedule.anchors.push(args);
            }
            return original(...args);
          };
        }
        return source;
      };
      const suspend = context.suspend.bind(context), resume = context.resume.bind(context);
      context.suspend = context.resume = context.close = () => Promise.resolve();
      Object.defineProperty(context, 'state', { get: () => 'running' });
      window.AudioContext = function () { return context; };
      const { default: init, Probe } = await import('/web_probe.js');
      await init();
      const probe = new Probe();
      try {
        probe.unlock();
        // The next task observes the completed resume promise and wasm futures.
        await new Promise(resolve => {
          const task = new MessageChannel();
          task.port1.onmessage = () => { task.port1.close(); task.port2.close(); resolve(); };
          task.port2.postMessage(null);
        });
        if (!probe.ready()) throw new Error('output did not become ready');
        if (player === 'full') probe.full_mix();
        const samples = Float32Array.from({ length: sourceFrames }, (_, i) =>
          sine === 'position' ? i / sourceFrames : sine ? Math.sin(2 * Math.PI * i / 480) * 0.5 : 1);
        const apply = update => {
          if (update.add) probe.add_voice();
          if (update.tick !== undefined) {
            probe.tick(update.tick, update.pitch ?? 0);
          }
          else if (update.pitch !== undefined) probe.pitch(update.pitch);
          if (update.left !== undefined) probe.set(update.left, update.right);
          observations.push({ tick: update.tick, cached: probe.cached_sounds() });
        };
        if (!player && !probe.start(samples, 48000)) throw new Error('voice refused');
        const pending = updates.filter(u => u.frame > 0).map(update => ({
          update, reached: suspend(update.frame / 48000),
        }));
        for (const update of updates.filter(u => u.frame === 0)) apply(update);
        const rendered = context.startRendering();
        for (const { update, reached } of pending) {
          await reached;
          if (context.currentTime !== update.frame / 48000) throw new Error('wrong suspend frame');
          apply(update);
          await resume();
        }
        const buffer = await rendered;
        const stoppedBeforeCleanup = stops;
        if (player) {
          if (player === 'full') {
            await ended[31]; // loops have priority; the finite source starts last
            probe.tick(900, 0);
          } else {
            await Promise.all(ended);
            probe.tick(21, 0);
          }
        }
        return { channels: [0, 1].map(channel => Array.from(buffer.getChannelData(channel))),
          starts, stops: stoppedBeforeCleanup, rates, rateSchedules, observations, cached: probe.cached_sounds() };
      } finally { probe.free(); }
    }, { frames, updates, sine, player, sourceFrames });
  } finally { await browser.close(); }
}

test('web voices reach their stereo gains linearly in ten milliseconds', async () => {
  const { channels } = await render(960, [{ frame: 0, left: 1, right: 0.5 }]);
  for (const [channel, target] of [1, 0.5].entries()) {
    const data = channels[channel];
    for (const frame of [0, 120, 240, 479, 480, 959]) {
      expect(data[frame]).toBeCloseTo(target * Math.min(frame / 480, 1), 5);
    }
    for (const length of [480, 960]) {
      const energy = data.slice(0, length).reduce((sum, value) => sum + value * value, 0);
      // Native advances its gain before each sample; Web Audio starts at zero.
      const nativeEnergy = Array.from({ length }, (_, i) => (target * Math.min((i + 1) / 480, 1)) ** 2)
        .reduce((sum, value) => sum + value, 0);
      expect(Math.abs(10 * Math.log10(energy / nativeEnergy))).toBeLessThan(0.03);
    }
  }
});

test('web gain retargeting holds the current value and replaces the old ramp', async () => {
  const updates = [
    { frame: 0, left: 0.25, right: 1 },
    { frame: 0, left: 1, right: 0.5 },
    { frame: 256, left: 0, right: 1 },
    { frame: 384, left: 0.75, right: 0.25 },
    { frame: 1024, left: 0, right: 0 },
  ];
  const { channels } = await render(1664, updates);
  for (const [channel, name] of ['left', 'right'].entries()) {
    let value = 0, step = 0, remaining = 0;
    for (let frame = 0; frame < channels[channel].length; frame++) {
      for (const update of updates.filter(u => u.frame === frame)) {
        step = (update[name] - value) / 480;
        remaining = 480;
      }
      expect(channels[channel][frame]).toBeCloseTo(value, 5);
      if (remaining > 0) { value += step; remaining--; }
    }
  }
});

test('web rate changes ease the playing source without another start or stop', async () => {
  const { channels, starts, stops, rates } = await render(16384, [
    { frame: 0, left: 1, right: 1 },
    { frame: 2048, pitch: 2 },
    { frame: 8192, pitch: 0.5 },
  ], true);
  expect(starts).toBe(1);
  expect(stops).toBe(0);
  expect(rates).toEqual([[2, 2048 / 48000, 0.01], [0.5, 8192 / 48000, 0.01]]);
  const period = values => {
    const crossings = [];
    for (let i = 1; i < values.length; i++) if (values[i - 1] < 0 && values[i] >= 0) crossings.push(i);
    return (crossings.at(-1) - crossings[0]) / (crossings.length - 1);
  };
  expect(Math.abs(period(channels[0].slice(480, 2048)) - 480)).toBeLessThan(1);
  expect(period(channels[0].slice(2048, 3200))).toBeGreaterThan(250);
  expect(period(channels[0].slice(2048, 3200))).toBeLessThan(400);
  expect(Math.abs(period(channels[0].slice(6000, 8192)) - 240)).toBeLessThan(1);
  expect(Math.abs(period(channels[0].slice(12000)) - 960)).toBeLessThan(1);
  for (let i = 2047; i < channels[0].length - 1; i++) {
    expect(Math.abs(channels[0][i + 1] - channels[0][i])).toBeLessThan(0.014);
  }
});

test('many web pitch retargets keep one approach from the current rate', async () => {
  const updates = Array.from({ length: 96 }, (_, i) => ({
    frame: (i + 1) * 128, pitch: i % 2 ? 0.5 : 2,
  }));
  const { channels, starts, stops, rates, rateSchedules } = await render(24576, [
    { frame: 0, left: 1, right: 1 }, ...updates,
  ], true);
  expect(starts).toBe(1);
  expect(stops).toBe(0);
  expect(rates.length).toBe(updates.length);
  const [schedule] = rateSchedules;
  expect(schedule.peak).toBe(2);
  expect(schedule.events.length).toBe(2);
  expect(schedule.anchors.length).toBe(updates.length);
  let from = 1, target = 1, at = 0;
  for (const [i, update] of updates.entries()) {
    const now = update.frame / 48000;
    const current = target + (from - target) * Math.exp(-(now - at) / 0.01);
    expect(schedule.anchors[i][0]).toBeCloseTo(current, 5);
    expect(schedule.anchors[i][1]).toBe(now);
    from = current;
    target = update.pitch;
    at = now;
  }
  const data = channels[0].slice(18000), crossings = [];
  for (let i = 1; i < data.length; i++) if (data[i - 1] < 0 && data[i] >= 0) crossings.push(i);
  const period = (crossings.at(-1) - crossings[0]) / (crossings.length - 1);
  expect(Math.abs(period - 960)).toBeLessThan(1);
});

test('a finite source finishing early frees a full web mix without restarting', async () => {
  const { starts, cached, observations } = await render(720128, [
    { frame: 0, tick: 0 },
    { frame: 2432, tick: 3, pitch: 0.01 },
    { frame: 614400, tick: 768 },
    { frame: 614528, tick: 768, add: true },
  ], false, 'full');
  expect(observations[2].cached).toBe(1);
  expect(starts).toBe(33);
  expect(cached).toBe(1);
});

test('web pitch lag is bounded for jumps from one to two and sixteen', async () => {
  for (const pitch of [2, 16]) {
    const frames = 16384, changed = 1024, measured = 15360;
    const { channels } = await render(frames, [
      { frame: 0, left: 1, right: 1 }, { frame: changed, pitch },
    ], 'position');
    const position = channels[0][measured] * frames * 32;
    const lag = changed + (measured - changed) * pitch - position;
    console.log(`web 1->${pitch}: source lag ${lag / 48} ms, wall lag ${lag / 48 / pitch} ms`);
    expect(lag / 48 / (pitch - 1)).toBeGreaterThan(9);
    expect(lag / 48 / (pitch - 1)).toBeLessThan(13);
  }
});
