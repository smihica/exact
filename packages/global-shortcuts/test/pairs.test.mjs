import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

test.skipIf(process.platform !== 'darwin')('modifier pairs follow the event state after missed events', () => {
  const root = resolve(import.meta.dir, '../../..');
  mkdirSync(join(root, 'target'), { recursive: true });
  const out = mkdtempSync(join(root, 'target/global-shortcut-pairs-'));
  const run = (command, args) => {
    const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout: 60_000 });
    assert.equal(result.status, 0, `${command}: ${result.error ?? result.signal ?? ''}\n${result.stdout}\n${result.stderr}`);
  };
  try {
    run('xcrun', ['swiftc', '-parse-as-library', '-emit-library', '-static', '-module-name', 'ExactPackage',
      '-emit-module', '-emit-module-path', join(out, 'ExactPackage.swiftmodule'),
      'host/apple/modules/ExactPackage.swift', '-o', join(out, 'libExactPackage.a')]);
    // A same-file extension reaches the private registry without changing the production source or installing monitors.
    const source = readFileSync(resolve(import.meta.dir, '../apple/GlobalShortcuts.swift'), 'utf8');
    const checks = readFileSync(resolve(import.meta.dir, 'Pairs.swift'), 'utf8');
    writeFileSync(join(out, 'Pairs.swift'), `${source}\n${checks}`);
    run('xcrun', ['swiftc', '-parse-as-library', '-I', out, '-L', out, '-lExactPackage',
      join(out, 'Pairs.swift'), '-o', join(out, 'pairs')]);
    run(join(out, 'pairs'), []);
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
}, 120_000);
