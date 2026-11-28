import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

// D7: a tag that is not structurally valid BCP 47 is refused, as the web's Translator and LanguageDetector refuse it
// with a RangeError, so `zh_tw` is no tag on Apple either (Android: TranslationTest.canonicalisesAsBcp47).
test.skipIf(process.platform !== 'darwin')('Apple canonicalises a tag as BCP 47 and refuses an underscored one', () => {
  const root = resolve(import.meta.dir, '../../..');
  mkdirSync(join(root, 'target'), { recursive: true });
  const out = mkdtempSync(join(root, 'target/translation-languages-'));
  const run = (command, args) => {
    const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout: 120_000 });
    assert.equal(result.status, 0, `${command}: ${result.error ?? result.signal ?? ''}\n${result.stdout}\n${result.stderr}`);
    return result.stdout;
  };
  try {
    writeFileSync(join(out, 'main.swift'), [
      'for tag in ["zh-tw", "SR-latn-rs", "zh_tw", "en_US", "sr-Latn_RS", "en--US", ""] {',
      '    let canonical = Languages.canonical(tag) ?? "nil"',
      '    print(tag + "=" + canonical)',
      '}',
      '',
    ].join('\n'));
    run('xcrun', ['swiftc', resolve(import.meta.dir, '../apple/Languages.swift'), join(out, 'main.swift'), '-o', join(out, 'languages')]);
    assert.equal(run(join(out, 'languages'), []), [
      'zh-tw=zh-TW', 'SR-latn-rs=sr-Latn-RS', 'zh_tw=nil', 'en_US=nil', 'sr-Latn_RS=nil', 'en--US=nil', '=nil', '',
    ].join('\n'));
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
}, 240_000);
