import { expect, test } from 'bun:test';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));

// verify runs `contract verify` beside the device run, and its signal listener took the place of the signal's
// default: Ctrl-C ended only that child, and verify carried the device run to its end. It ends now, with the code.
test('a signal while contract verify runs ends verify with the signal\'s code', async () => {
  const script = `import { contractVerify } from './scripts/verify-semantics.mjs';
contractVerify({ dir: '/nonexistent-app' }, false);
setInterval(() => {}, 1000); // the device run, still going`;
  const child = spawn(process.execPath, ['-e', script], { cwd: ROOT, stdio: 'ignore' });
  await Bun.sleep(500);
  const ended = new Promise((done) => child.on('exit', (code, signal) => done({ code, signal })));
  child.kill('SIGINT');
  const result = await Promise.race([ended, Bun.sleep(15000).then(() => 'still running')]);
  if (result === 'still running') child.kill('SIGKILL');
  expect(result).toEqual({ code: 130, signal: null });
}, 30000);
