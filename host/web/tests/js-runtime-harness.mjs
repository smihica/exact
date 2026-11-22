// The JS target's runtime in a `web-js` directory, as built, for the suites that drive it in Bun: the modules they
// run, beside stand-ins for the rest (stand-ins.mjs). A suite writes its own stubs into `dir` before it imports
// anything from there: the loader reads the directory once.
import { afterAll } from 'bun:test';
import { copyFileSync, cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { standIns } from './stand-ins.mjs';

export const webJs = name => resolve(fileURLToPath(new URL(`../../web-js/${name}`, import.meta.url)));

const REAL = ['commands.js', 'rt.js', 'format.js', 'clock.js', 'resources.js', 'tree.js', 'events.js', 'pieces.js', 'mount.js', 'strings.js', 'request-send.js', 'presence-bindings.js', 'rows.js', 'uploads.js', 'held.js', 'font-family.js', 'observers.js', 'grid.js', 'overlay.js', 'roster.js', 'router.js', 'route-pattern.js', 'schedule.js', 'budget.js', 'shape.js', 'modules.js', 'permissions.js', 'samples.js', 'spans.js', 'update.js', 'arrival.js', 'devices.js', 'wakes.js', 'reach.js', 'members.js', 'resource-inspection.js', 'sheets.js'];
// The stand-ins export what these modules and ts-data.js import; the values the tests read are stated.
const STATED = {
  'admission.js': { createSecretFacade: '() => ({ read: false })', hasGrant: '() => true', appGrantSet: '() => null', coversPath: '() => true',
    setAppGrantSet: 'g => g', overlaying: '{ on: false }', inOverlay: 'api => new Error(api)' },
  'admission-data.js': { tsGrantSet: '{}' },
  // The build points ts-data.js's `__FETCH_MODULE__` here, from which it imports `fetch`.
  'ts-fetch.js': { fetch: '() => {}', answering: '{ call: null, untraced: new Set(), ordinary: (start) => start() }', files: '{ appId: null }', activeCall: '() => answering.call',
    SOCKET_AND_FETCH: "'a mutation cannot both send on a socket and fetch'" },
  'names.js': { sourceTypes: '{}' },
};
/**
 * The runtime in a directory of the calling test file's own, removed after its tests. Bun shares this module across
 * the files of one run, and runs each file's tests before it loads the next, so a directory made once here would be
 * gone before a second file used it.
 */
export function runtimeHarness() {
  // modules.js imports the web host's `../web/address-space.js` and mount.js its `../web/faults.js`, stood in
  const root = mkdtempSync(resolve(tmpdir(), 'exact-js-baked-')), dir = resolve(root, 'web-js');
  afterAll(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(dir);
  mkdirSync(resolve(root, 'web'));
  writeFileSync(resolve(root, 'web/address-space.js'), 'export const setSpaceState = () => {};');
  writeFileSync(resolve(root, 'web/faults.js'), 'export const checkpointFaults = () => [];');
  for (const f of REAL) copyFileSync(webJs(f), resolve(dir, f));
  for (const [file, source] of standIns([...REAL, 'ts-data.js'], STATED)) writeFileSync(resolve(dir, file), source);
  // A view transition that holds every tree update (shared.js's commit returns before its callback).
  writeFileSync(resolve(dir, 'shared.js'), 'export const commit = (tail) => { globalThis.heldTail = tail; return true; };');
  writeFileSync(resolve(dir, 'presence-glue.js'), 'globalThis.exact.presence = () => ({ before() {}, after() {}, exit() {} });');
  /** A copy of the runtime for one test (`tag`), so what that test sets reaches no other. */
  const freshRuntime = tag => {
    const own = resolve(root, `web-js-${tag}`);
    cpSync(dir, own, { recursive: true });
    return import(resolve(own, 'rt.js'));
  };
  return { root, dir, freshRuntime };
}
