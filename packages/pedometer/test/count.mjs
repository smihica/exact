// Native pedometer boundaries and sums (LLP 1133.026 D12): Swift on a Mac, and Kotlin compiled by the compiler
// Gradle keeps in its cache, which the Android build downloaded, and run on the JDK.
//
// usage: bun packages/pedometer/test/count.mjs [--apple|--android]
// Android needs a JDK 17 or newer in JAVA_HOME, and one Android build done.
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const here = import.meta.dir, pkg = resolve(here, '..'), root = resolve(pkg, '../..');
const target = resolve(root, 'target');
mkdirSync(target, { recursive: true });
if (process.platform === 'darwin' && !process.argv.includes('--android')) {
  const out = mkdtempSync(join(target, 'pedometer-boundary-'));
  try {
    const commands = [
      ['swiftc', ['-parse-as-library', '-emit-library', '-static', '-module-name', 'ExactPackage', '-emit-module',
        '-emit-module-path', join(out, 'ExactPackage.swiftmodule'), resolve(root, 'host/apple/modules/ExactPackage.swift'),
        '-o', join(out, 'libExactPackage.a')]],
      ['swiftc', ['-parse-as-library', '-I', out, '-L', out, '-lExactPackage', resolve(pkg, 'apple/Pedometer.swift'),
        resolve(here, 'count/BoundaryCheck.swift'), '-o', join(out, 'boundary')]],
      [join(out, 'boundary'), []],
    ];
    for (const [command, args] of commands) {
      const result = spawnSync(command, args, { stdio: 'inherit' });
      if (result.status !== 0) { process.exitCode = result.status ?? 1; break; }
    }
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
}
if (process.argv.includes('--apple')) process.exit(process.exitCode ?? 0);
const cache = resolve(process.env.GRADLE_USER_HOME ?? join(homedir(), '.gradle'), 'caches/modules-2/files-2.1');
/** The newest cached jar of `group:name`, or of `group:name:version`. */
const jar = (group, name, version) => {
  const dir = resolve(cache, group, name);
  const versions = existsSync(dir) ? readdirSync(dir).filter((v) => !version || v === version).sort((a, b) => a.localeCompare(b, 'en', { numeric: true })) : [];
  for (const v of versions.reverse()) {
    for (const hash of readdirSync(resolve(dir, v))) {
      const file = readdirSync(resolve(dir, v, hash)).find((f) => f === `${name}-${v}.jar`);
      if (file) return { path: resolve(dir, v, hash, file), version: v };
    }
  }
  throw new Error(`no ${group}:${name}${version ? `:${version}` : ''} in ${cache}: build an Android app once (bun scripts/agent-android.mjs build <app>)`);
};
const compiler = jar('org.jetbrains.kotlin', 'kotlin-compiler-embeddable');
const stdlib = jar('org.jetbrains.kotlin', 'kotlin-stdlib', compiler.version).path;
const classpath = [compiler.path, stdlib, jar('org.jetbrains.kotlin', 'kotlin-script-runtime', compiler.version).path,
  jar('org.jetbrains.kotlin', 'kotlin-reflect', compiler.version).path, jar('org.jetbrains.kotlin', 'kotlin-daemon-embeddable', compiler.version).path,
  jar('org.jetbrains.kotlinx', 'kotlinx-coroutines-core-jvm').path, jar('org.jetbrains', 'annotations').path].join(':');
const java = process.env.JAVA_HOME ? resolve(process.env.JAVA_HOME, 'bin/java') : 'java';
const out = mkdtempSync(join(target, 'pedometer-count-'));
try {
  const built = spawnSync(java, ['-cp', classpath, 'org.jetbrains.kotlin.cli.jvm.K2JVMCompiler', '-no-stdlib', '-no-reflect', '-cp', stdlib, '-d', out,
    resolve(pkg, 'android/Count.kt'), resolve(pkg, 'android/Subscription.kt'),
    resolve(here, 'count/CountCheck.kt')], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stdout + built.stderr);
    process.exit(1);
  }
  const ran = spawnSync(java, ['-cp', `${out}:${stdlib}`, 'CountCheckKt'], { stdio: 'inherit' });
  process.exitCode ||= ran.status ?? 1;
} finally {
  rmSync(out, { recursive: true, force: true });
}
