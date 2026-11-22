// The parity comparator, generalised from scripts/svgparity.mjs (LLP 1055.000
// §5, LLP 1056 §4): a gallery app's `fx-*` boxes cropped from each host's
// screenshots by that host's own layout and compared with Chrome's.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { appleArtifacts, appleRunBinary } from '../host/apple/build.mjs';
import { linuxRebuild, ownVerb, rebuildCommand } from './agent-build.mjs';
import { linuxBinary, resolveApp } from './app.mjs';
import { crop, readSrgbPng, shrink } from './png.mjs';

const ROOT = resolve(import.meta.dir, '..');

/** Report every missing requested native build before starting a parity capture. */
export function requireNativeBuilds(app, hosts, { device = false } = {}) {
  const a = resolveApp(app), missing = [];
  for (const host of new Set(hosts)) {
    const override = host === 'linux' ? 'EXACT_LINUX_BIN' : host === 'macos' ? 'EXACT_MAC_BIN' : null;
    const built = host === 'linux' ? (process.env.EXACT_LINUX_BIN ?? linuxBinary(a))
      : host === 'macos' ? (process.env.EXACT_MAC_BIN ?? appleRunBinary(a))
      : host === 'ios' ? appleArtifacts(a, { destination: device ? 'ios' : 'ios-simulator' }).bundle : null;
    if (built === null || existsSync(built)) continue;
    if (override && process.env[override] !== undefined) {
      missing.push(`  ${host}: ${override} names no file: ${built}`);
      continue;
    }
    const command = host === 'linux' ? linuxRebuild(a)
      : host === 'ios' && device ? (ownVerb(a, 'ios --device') ?? `bun host/apple/build.mjs --device ${a.crate('apple')}`)
      : rebuildCommand(a, host === 'ios' ? 'ios' : 'mac');
    missing.push(`  ${host}: run ${command} first`);
  }
  if (missing.length) throw new Error(`missing native builds for ${a.name}:\n${missing.join('\n')}`);
}

/** A gallery's web build, the JS target the app ships (LLP 1071), in its own dist (`target/web-dist-js/<app>`) so a
 * smoke leaves `host/web/dist` alone; built when missing, and again under `EXACT_PARITY_REBUILD=1`. */
export function webBuild(app) {
  const dist = resolve(ROOT, 'target/web-dist-js', app);
  if (process.env.EXACT_PARITY_REBUILD === '1' || !existsSync(resolve(dist, '.exact-build.json'))) {
    mkdirSync(dirname(dist), { recursive: true });
    const b = spawnSync('bun', [resolve(ROOT, 'host/web/build.mjs'), `${app}-web`, '--js'], { cwd: ROOT, env: { ...process.env, EXACT_WEB_DIST: dist }, stdio: 'inherit' });
    if (b.status !== 0) throw new Error(`the ${app} web build failed`);
  }
  return dist;
}

/** Mean |Δ| over RGB and the share of pixels off by more than `band`, `b`
 * shifted by (dx, dy) over `a`'s interior (`slop` px of margin). */
export function compare(a, b, dx, dy, { slop, band }) {
  let sum = 0, off = 0, n = 0;
  for (let y = slop; y < a.height - slop; y++) for (let x = slop; x < a.width - slop; x++) {
    const i = (y * a.width + x) * 4, j = ((y + dy) * b.width + x + dx) * 4;
    let m = 0;
    for (let c = 0; c < 3; c++) { const d = Math.abs(a.data[i + c] - b.data[j + c]); sum += d; if (d > m) m = d; }
    if (m > band) off++;
    n++;
  }
  return { mean: sum / (3 * n), off: off / n };
}

/** The best registration of `b` over `a` within ±slop. */
export function register(a, b, { slop, band }) {
  let best = null;
  for (let dy = -slop; dy <= slop; dy++) for (let dx = -slop; dx <= slop; dx++) {
    const r = compare(a, b, dx, dy, { slop, band });
    if (!best || r.mean < best.mean) best = { ...r, dx, dy };
  }
  return best;
}

/** One sRGB screenshot with its device scale `k` and the viewport's origin
 * in the image. `film` samples finite motion at the clock (LLP 1116.003 D8). */
export async function shot(s, host, dir, name, film = false) {
  const path = resolve(dir, `${host}-${name}.png`);
  const reply = await s.screenshot(path, host === 'macos', undefined, film);
  if (reply.captureSpace === 'screen' && reply.scale === null) throw new Error(`${host} parity: ${reply.note}`);
  const layout = await s.layout();
  const img = readSrgbPng(path);
  if (reply.captureSpace === 'screen') {
    const { x, y } = layout.screen ?? {};
    if (!Number.isFinite(x) || !Number.isFinite(y)) throw new Error(`${host} parity: the viewport has no screen origin`);
    return { img, k: reply.scale ?? img.width / reply.w, screen: { w: reply.w, h: reply.h }, origin: { x, y }, layout };
  }
  const k = Math.round(img.width / layout.viewport.w);
  const top = Math.max(0, Math.round(img.height / k - layout.viewport.h));
  return { img, k, origin: { x: 0, y: top }, layout };
}

/** Every box whose testId matches `prefix`, cropped from a shot at 1 px per
 * point (`native` keeps the device scale), `slop` px of margin around it. */
export function boxes({ img, k, screen, origin, layout }, { prefix = 'fx-', slop, native = false }) {
  const sx = native ? (screen ? img.width / screen.w : k) : 1;
  const sy = native ? (screen ? img.height / screen.h : k) : 1;
  const w1 = Math.round(screen?.w ?? Math.floor(img.width / k));
  const h1 = Math.round(screen?.h ?? Math.floor(img.height / k));
  const full = native || (w1 === img.width && h1 === img.height) ? img : shrink(img, w1, h1);
  const out = {};
  for (const n of layout.nodes.filter((n) => n.testId?.startsWith(prefix))) {
    const left = n.x + origin.x, top = n.y + origin.y;
    const x = Math.round(left * sx), y = Math.round(top * sy);
    const w = native && screen ? Math.round((left + n.w) * sx) - x : Math.round(n.w * sx);
    const h = native && screen ? Math.round((top + n.h) * sy) - y : Math.round(n.h * sy);
    out[n.testId] = x < slop || y < slop || x + w + slop > full.width || y + h + slop > full.height
      ? null : crop(full, x - slop, y - slop, w + 2 * slop, h + 2 * slop);
  }
  return out;
}
