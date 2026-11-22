// Parity smoke prerequisites, and SVG fixtures declared undrawn by the schema's `hosts` column (LLP 1159.000 D6).
import { expect, test } from 'bun:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { boxes, shot } from './parity.mjs';
import { encodePng } from './png.mjs';
import { undrawn } from './svgparity.mjs';

test('parity crops a display capture at the viewport’s screen origin', async () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'exact-parity-screen-'));
  const width = 24, height = 32, data = new Uint8Array(width * height * 4).fill(255);
  const node = { testId: 'fx-tile', x: 1, y: 2, w: 2, h: 3 };
  const screen = { x: 3, y: 5, w: 12, h: 16 };
  for (let y = 14; y < 20; y++) {
    for (let x = 8; x < 12; x++) data.set([0, 0, 0, 255], (y * width + x) * 4);
  }
  const session = {
    layout: async () => ({ viewport: { w: 8, h: 9 }, screen, nodes: [node] }),
    screenshot: async path => {
      writeFileSync(path, encodePng({ width, height, data }));
      return { captureSpace: 'screen', w: 12, h: 16, scale: 2 };
    },
  };
  try {
    const pictured = await shot(session, 'fixture', dir, 'tile');
    const tile = boxes(pictured, { slop: 0 })['fx-tile'];
    expect(tile).toMatchObject({ width: 2, height: 3 });
    expect([...tile.data]).toEqual(Array.from({ length: 6 }, () => [0, 0, 0, 255]).flat());
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test('parity keeps a tile at the edge of a nonintegral display scale', async () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'exact-parity-edge-'));
  const width = 1080, height = 2340, w = 375, h = 812;
  const node = { testId: 'fx-edge', x: 365, y: 807, w: 5, h: 5 };
  const data = new Uint8Array(width * height * 4).fill(255);
  // The last native row lands at y=2339 when the two pixel ratios are used.
  for (let x = 1051; x < 1066; x++) data.set([0, 0, 0, 255], ((height - 1) * width + x) * 4);
  const session = {
    layout: async () => ({ viewport: { w, h }, screen: { x: 0, y: 0, w, h }, nodes: [node] }),
    screenshot: async path => {
      writeFileSync(path, encodePng({ width, height, data }));
      return { captureSpace: 'screen', w, h };
    },
  };
  try {
    const pictured = await shot(session, 'fixture', dir, 'tile');
    const tile = boxes(pictured, { slop: 0 })['fx-edge'];
    expect(tile).toMatchObject({ width: 5, height: 5 });
    const native = boxes(pictured, { slop: 0, native: true })['fx-edge'];
    expect(native).toMatchObject({ width: 15, height: 14 });
    expect([...native.data.slice(-4)]).toEqual([0, 0, 0, 255]);
  } finally { rmSync(dir, { recursive: true, force: true }); }
});

test('a fixture is excused where the schema says the row is not drawn, and nowhere else', () => {
  expect(undrawn('ios')).toEqual([]);
  expect(undrawn('macos')).toEqual([]);
  // The paint walk's gap is every host's that draws with it.
  // SVG blends include their backdrop, including on Android's recording path.
  for (const host of ['linux', 'windows']) expect(undrawn(host)).toEqual([]);
  expect(undrawn('android')).toEqual([]);
  const styles = [
    { field: 'mix_blend_mode', hosts: [{ host: 'ios', fidelity: 'best-effort', why: 'x' }] },
    { field: 'filter', hosts: [{ host: 'linux', on: 'svg', fidelity: 'none', why: 'x' }] },
  ];
  // `best-effort` is compared, and an entry for another scope excuses nothing.
  expect(undrawn('ios', { styles })).toEqual([]);
  expect(undrawn('linux', { styles })).toEqual([]);
  // A gap in the paint walk is every host's that draws with it, on the scope it names.
  const walk = { styles: [{ field: 'filter', hosts: [{ host: 'painted', on: 'box', fidelity: 'none', why: 'x' }] }] };
  for (const host of ['linux', 'windows', 'android']) expect(undrawn(host, walk)).toEqual(['fx-box-filter', 'fx-image-filter']);
  expect(undrawn('ios', walk)).toEqual([]);
});

for (const smoke of ['svg', 'motion']) {
  for (const scenario of [
    { name: 'all builds missing', hosts: ['linux', 'macos', 'ios'], present: [] },
    { name: 'only Linux built', hosts: ['linux', 'macos', 'ios'], present: ['linux'] },
    { name: 'only Apple built', hosts: ['linux', 'macos', 'ios'], present: ['macos', 'ios'] },
    { name: 'all builds present', hosts: ['linux', 'macos', 'ios'], present: ['linux', 'macos', 'ios'] },
    { name: 'only Linux requested', hosts: ['linux'], present: [] },
    { name: 'only Apple requested', hosts: ['ios', 'macos'], present: [] },
    { name: 'unrequested builds missing', hosts: ['linux'], present: ['linux'] },
    { name: 'debug is not host-dev', hosts: ['linux'], present: [], debug: true },
    { name: 'binary overrides present', hosts: ['linux', 'macos', 'ios'], present: ['linux', 'macos', 'ios'], overrides: ['linux', 'macos'] },
    { name: 'macOS override missing', hosts: ['macos'], present: [], overrides: ['macos'] },
    { name: 'macOS override missing with other builds', hosts: ['linux', 'macos', 'ios'], present: [], overrides: ['macos'] },
    { name: 'Linux override missing with other builds', hosts: ['linux', 'macos', 'ios'], present: [], overrides: ['linux'] },
    { name: 'Linux override empty', hosts: ['linux'], present: [], overrides: ['linux'], emptyOverrides: true },
    { name: 'macOS override empty', hosts: ['macos'], present: [], overrides: ['macos'], emptyOverrides: true },
    { name: 'binary overrides empty with other builds missing', hosts: ['linux', 'macos', 'ios'], present: [], overrides: ['linux', 'macos'], emptyOverrides: true },
    { name: 'binary overrides missing despite default builds', hosts: ['linux', 'macos', 'ios'], present: ['ios'], overrides: ['linux', 'macos'], defaults: true },
    { name: 'unrequested override missing', hosts: ['linux'], present: ['linux'], overrides: ['macos'] },
    { name: 'a simulator build is not a phone build', hosts: ['ios'], present: [], device: true, simulator: true },
    { name: 'phone build present', hosts: ['ios'], present: ['ios'], device: true },
    { name: 'default hosts', present: [] },
  ]) {
    test(`${smoke} smoke prerequisites: ${scenario.name}`, () => {
      // Keep module mocks in a child, as the real parity modules are also tested in this process.
      const script = `
        import assert from 'node:assert/strict';
        import { mock } from 'bun:test';
        import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
        import { dirname, resolve } from 'node:path';
        import { appleArtifacts, appleRunBinary } from './host/apple/build.mjs';
        import { linuxBinary, resolveApp } from './scripts/app.mjs';
        import * as agent from './scripts/agent.mjs';
        const scenario = ${JSON.stringify(scenario)}, smoke = ${JSON.stringify(smoke)};
        const root = resolve('target');
        mkdirSync(root, { recursive: true });
        const target = mkdtempSync(resolve(root, 'parity-prerequisites-'));
        try {
          for (const key of ['EXACT_APP_DIR', 'EXACT_MAC_BIN', 'EXACT_LINUX_BIN', 'EXACT_UPDATE_TRUST']) delete process.env[key];
          process.env.CARGO_TARGET_DIR = target;
          const app = resolveApp(smoke + '-gallery');
          const hosts = scenario.hosts ?? ['linux', ...(process.platform === 'darwin' ? ['macos', 'ios'] : [])];
          const artifacts = {
            linux: linuxBinary(app),
            macos: appleRunBinary(app),
            ios: appleArtifacts(app, { destination: scenario.device ? 'ios' : 'ios-simulator' }).bundle,
          };
          const overrides = { linux: 'EXACT_LINUX_BIN', macos: 'EXACT_MAC_BIN' };
          for (const host of scenario.overrides ?? []) {
            process.env[overrides[host]] = artifacts[host] = scenario.emptyOverrides ? '' : resolve(target, 'override-' + host);
          }
          const write = path => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, ''); };
          if (scenario.defaults) { write(linuxBinary(app)); write(appleRunBinary(app)); }
          for (const host of scenario.present) {
            if (host === 'ios') mkdirSync(artifacts.ios, { recursive: true });
            else write(artifacts[host]);
          }
          if (scenario.debug) write(resolve(target, 'debug', app.crate('linux')));
          if (scenario.simulator) mkdirSync(appleArtifacts(app, { destination: 'ios-simulator' }).bundle, { recursive: true });
          const commands = {
            linux: 'bun scripts/build-linux.mjs ' + app.name,
            macos: 'bun host/apple/build.mjs ' + app.crate('apple'),
            ios: 'bun host/apple/build.mjs ' + (scenario.device ? '--device ' : '--ios ') + app.crate('apple'),
          };
          const missing = hosts.filter(host => !scenario.present.includes(host));
          const reached = new Error('web reference reached'), opened = [];
          mock.module('./scripts/agent.mjs', () => ({ ...agent,
            open: async options => { opened.push(options); throw reached; },
          }));
          mock.module('./scripts/' + smoke + 'parity.mjs', () => ({
            [smoke + 'Parity']: async options => {
              assert.deepEqual(options.hosts, hosts);
              await options.open({ host: 'web', app: app.name });
            },
          }));
          process.argv = [process.execPath, './scripts/smoke.mjs', smoke,
            ...(scenario.hosts ? ['--hosts', hosts.join(',')] : []), ...(scenario.device ? ['--device'] : [])];
          try { await import('./scripts/smoke.mjs'); assert.fail('smoke did not reach its capture'); }
          catch (error) {
            if (missing.length) {
              assert.equal(error.message, 'missing native builds for ' + app.name + ':\\n'
                + missing.map(host => '  ' + host + ': ' + (scenario.overrides?.includes(host)
                  ? overrides[host] + ' names no file: ' + artifacts[host]
                  : 'run ' + commands[host] + ' first')).join('\\n'));
              assert.deepEqual(opened, []);
            } else {
              assert.equal(error, reached);
              assert.equal(opened.length, 1);
              assert.equal(opened[0].host, 'web');
              assert.equal(opened[0].app, app.name);
            }
          }
        } finally { rmSync(target, { recursive: true, force: true }); }
        console.log('prerequisites checked');
      `;
      const result = spawnSync(process.execPath, ['-e', script], {
        cwd: resolve(import.meta.dir, '..'), encoding: 'utf8', timeout: 20_000,
      });
      expect({ status: result.status, output: result.stdout + result.stderr, error: result.error?.message })
        .toEqual({ status: 0, output: 'prerequisites checked\n', error: undefined });
    });
  }
}
