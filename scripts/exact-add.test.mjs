// `exact add`'s install step: the package manager an app's tree names (scripts/package-manager.mjs).
import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, renameSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';

function inDir(body) {
  const dir = mkdtempSync(resolve(tmpdir(), 'exact-add-'));
  try { return body(dir); } finally { rmSync(dir, { recursive: true, force: true }); }
}

test('add installs with the package manager the app\'s tree names, or with none under --no-install', async () => {
  const { packageManager } = await import('./package-manager.mjs');
  inDir(dir => {
    const app = resolve(dir, 'workspace/apps/web');
    mkdirSync(app, { recursive: true });
    mkdirSync(resolve(dir, '.git'));
    assert.equal(packageManager(app), 'bun', 'none says: Bun, stopping at the repository');
    writeFileSync(resolve(dir, 'workspace/pnpm-lock.yaml'), '');
    assert.equal(packageManager(app), 'pnpm', 'the nearest lockfile');
    writeFileSync(resolve(app, 'package.json'), JSON.stringify({ packageManager: 'yarn@4.5.0' }));
    assert.equal(packageManager(app), 'yarn', 'a packageManager field');
  });
  inDir(dir => {
    // A pnpm workspace that says so at its root, with a bun.lock left over in the app.
    const app = resolve(dir, 'apps/web');
    mkdirSync(app, { recursive: true });
    mkdirSync(resolve(dir, '.git'));
    writeFileSync(resolve(dir, 'package.json'), JSON.stringify({ packageManager: 'pnpm@9.12.0' }));
    writeFileSync(resolve(app, 'package.json'), JSON.stringify({ name: 'web' }));
    writeFileSync(resolve(app, 'bun.lock'), '');
    assert.equal(packageManager(app), 'pnpm', 'a declaration wins over a nearer lockfile');
  });
  inDir(dir => {
    const root = resolve(import.meta.dir, '..'), app = resolve(dir, 'shop');
    const made = spawnSync(process.execPath, [resolve(root, 'scripts/exact.mjs'), 'new', app], { encoding: 'utf8' });
    assert.equal(made.status, 0, made.stdout + made.stderr);
    writeFileSync(resolve(dir, 'pnpm-lock.yaml'), '');
    const added = spawnSync(process.execPath, ['exact.mjs', 'add', 'convex', '--no-install'], { cwd: app, encoding: 'utf8', env: { ...process.env, EXACT: root } });
    assert.equal(added.status, 0, added.stdout + added.stderr);
    assert.match(added.stdout, /@exact\/convex is file:.*, not installed: run pnpm install\./);
    assert.ok(JSON.parse(readFileSync(resolve(app, 'package.json'), 'utf8')).dependencies['@exact/convex']);
    assert.ok(!existsSync(resolve(app, 'node_modules/@exact/convex')));
  });
});


// #427: a package outside exact2 is added by its directory, as `file:` with a path from the app, or by a registry name
// with its scope or version; a bare name stays exact2's own.
test('add takes an outside package by its directory and names its GPU surfaces', async () => {
  const { outsideSpecifier } = await import('./package-manager.mjs');
  for (const spec of ['../maplibre-exact', './map', '/abs/map', 'file:../map', '@birkskyum/maplibre-exact', 'maplibre-exact@1.2.0']) assert.ok(outsideSpecifier(spec), spec);
  for (const spec of ['geolocaton', 'maplibre-exact', undefined]) assert.ok(!outsideSpecifier(spec), String(spec));
  inDir(dir => {
    const root = resolve(import.meta.dir, '..'), app = resolve(dir, 'shop'), pkg = resolve(dir, 'sky');
    cpSync(resolve(root, 'packages/fixtures/sky'), pkg, { recursive: true });
    const made = spawnSync(process.execPath, [resolve(root, 'scripts/exact.mjs'), 'new', app], { encoding: 'utf8' });
    assert.equal(made.status, 0, made.stdout + made.stderr);
    const added = spawnSync(process.execPath, ['exact.mjs', 'add', '../sky'], { cwd: app, encoding: 'utf8', env: { ...process.env, EXACT: root } });
    assert.equal(added.status, 0, added.stdout + added.stderr);
    assert.match(added.stdout, /add: @fixture\/sky is file:\.\.\/sky\. Place its surfaces with canvas surface=sky\(…\)/);
    assert.equal(JSON.parse(readFileSync(resolve(app, 'package.json'), 'utf8')).dependencies['@fixture/sky'], 'file:../sky');
    assert.ok(existsSync(resolve(app, 'node_modules/@fixture/sky/exact.json')));
  });
});

test('add resolves a relative package from the physical working directory before preserving its spelling', () => inDir(dir => {
  const parent = realpathSync(dir), repo = resolve(parent, 'repo'), app = resolve(repo, 'mobile');
  const links = resolve(parent, 'links'), alias = resolve(links, 'mobile'), local = resolve(repo, 'local');
  for (const path of [app, links, local]) mkdirSync(path, { recursive: true });
  assert.equal(spawnSync('git', ['init', '-q', repo]).status, 0);
  symlinkSync(app, alias, 'dir');
  writeFileSync(resolve(local, 'package.json'), '{"name":"local","version":"1.0.0"}');
  const source = `import { addOutside } from ${JSON.stringify(resolve(import.meta.dir, 'package-manager.mjs'))};
    addOutside(${JSON.stringify(app)}, process.argv[1], false);`;
  for (const decoy of [false, true]) {
    if (decoy) {
      mkdirSync(resolve(links, 'local'));
      writeFileSync(resolve(links, 'local/package.json'), '{"name":"wrong-package","version":"1.0.0"}');
    }
    for (const spec of ['../local', 'file:../local']) {
      writeFileSync(resolve(app, 'package.json'), '{}');
      const added = spawnSync(process.execPath, ['-e', source, spec], {
        cwd: alias, env: { ...process.env, PWD: alias, EXACT_APP_SPELLING: alias }, encoding: 'utf8',
      });
      assert.equal(added.status, 0, added.stdout + added.stderr);
      assert.deepEqual(JSON.parse(readFileSync(resolve(app, 'package.json'), 'utf8')).dependencies, { local: 'file:../local' });
    }
  }
}));

for (const selector of ['argument', 'EXACT_APP_DIR']) for (const restart of [false, true]) {
  test(`direct add preserves the app's ${selector} spelling${restart ? ' across a Bun restart' : ''}`, () => inDir(dir => {
    const parent = realpathSync(dir), physical = resolve(parent, 'physical'), alias = resolve(parent, 'alias');
    const root = resolve(import.meta.dir, '..'), app = resolve(physical, 'mobile');
    mkdirSync(physical);
    symlinkSync(physical, alias, 'dir');
    symlinkSync(root, resolve(physical, 'sdk'), 'dir');
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('EXACT') && !key.startsWith('GIT_')));
    env.EXACT_FEEDBACK = '0';
    env.BUN = process.execPath;
    const run = (args, extra = {}) => {
      const result = spawnSync(process.execPath, ['alias/sdk/scripts/exact.mjs', ...args], {
        cwd: parent, env: { ...env, ...extra }, encoding: 'utf8', timeout: 30_000,
      });
      assert.equal(result.status, 0, result.stdout + result.stderr);
    };
    run(['new', 'alias/mobile']);
    assert.equal(spawnSync('git', ['init', '-q', app], { env }).status, 0);
    if (restart) {
      env.BUN = resolve(parent, process.platform === 'win32' ? 'bun.exe' : 'bun');
      copyFileSync(process.execPath, env.BUN);
    }
    run(['add', 'motion', ...(selector === 'argument' ? ['alias/mobile'] : []), '--no-install'],
      selector === 'EXACT_APP_DIR' ? { EXACT_APP_DIR: 'alias/mobile' } : {});
    const manifest = JSON.parse(readFileSync(resolve(app, 'package.json'), 'utf8'));
    assert.equal(manifest.dependencies['@exact/motion'], 'file:.exact/checkout/packages/motion');
    const moved = resolve(parent, 'moved');
    renameSync(physical, moved);
    rmSync(alias);
    const reference = manifest.dependencies['@exact/motion'].slice('file:'.length);
    assert.equal(realpathSync(resolve(moved, 'mobile', reference)), resolve(root, 'packages/motion'));
    const schema = JSON.parse(readFileSync(resolve(moved, 'mobile/app.json'), 'utf8')).$schema;
    assert.equal(realpathSync(resolve(moved, 'mobile', schema)), resolve(root, 'scripts/app.schema.json'));
  }), 30_000);
}
