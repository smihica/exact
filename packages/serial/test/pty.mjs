// @exact/serial bound on macOS or the Linux host headless (LLP 1133.037 §16 stages 2 and 5, Bench drive 10): Bench with
// the Swift or the Rust implementation answering (`--bind serial`), the port a pseudo-terminal this script makes and names through D19 item 3's
// `EXACT_DEVICE_FIXTURE` (a development build's injected device). The pty is set raw and filled with a megabyte of
// numbered lines before Bench opens it; the writes block while nothing reads, so the bytes wait in the kernel as a
// device's would, and Bench must read every one, in order, through the module's termios, read loop and bound.
// Bench also sends a line larger than the pty's buffer while nothing reads the other end, as a device that takes no
// bytes would: the reads go on, the write answers only once this script reads the line, and a second such write,
// never read, leaves Disconnect free to close the port.
// A pty carries no modem lines, so signals are not checked here.
//
// usage: bun packages/serial/test/pty.mjs [macos|linux]
//   (after `bun host/apple/build.mjs bench-apple`, or `cargo build --profile host-dev -p bench-linux`)
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { checker, root } from './chromium.mjs';

const LINES = 8192, WIDTH = 128, SENT = 64 * 1024;
const host = process.argv[2] ?? 'macos';
const { check, failed } = checker(host);
const dir = mkdtempSync(join(tmpdir(), 'serial-pty-'));
// The pty: raw on its slave side (held open here), the master written in full. Then each line on stdin names a count
// of bytes to read from the master, what Bench wrote, and the script answers with the bytes it read.
const writer = spawn('python3', ['-I', '-c', `
import os, sys, tty
master, slave = os.openpty()
tty.setraw(slave)
print(os.ttyname(slave), flush=True)
for i in range(${LINES}):
    line = ('line %05d ' % i).ljust(${WIDTH - 2}, '.') + '\\r\\n'
    os.write(master, line.encode())
for want in sys.stdin:
    got = b''
    while len(got) < int(want):
        got += os.read(master, int(want) - len(got))
    print(got.decode('latin-1').replace('\\r', 'CR').replace('\\n', 'LF'), flush=True)
`], { stdio: ['pipe', 'pipe', 'inherit'] });
const fromWriter = createInterface({ input: writer.stdout })[Symbol.asyncIterator]();
const path = (await fromWriter.next()).value.trim();
const fixture = join(dir, 'devices.json');
writeFileSync(fixture, JSON.stringify({ serial: [{ id: 'pty', name: 'Fixture port', path }] }));

const send = [['', `type send ${'x'.repeat(SENT)}`], ['', 'type send key Enter'], ['', 'clock settle']];
const steps = [['', 'clock settle'], ['ports', 'state ports'], ['', 'type port pty'], ['', 'clock settle'], ['', 'tap connect'], ['', 'clock settle'],
  // A line the other end does not read, then the reads.
  ...send, ...Array.from({ length: 30 }, () => ['', 'clock +500 real']), ['', 'clock settle'], ['log', 'state log'], ['conn', 'state conn'], ['unread', 'state sent'],
  // This script reads the line once `unread` answers; the write then answers.
  ['', 'clock +500 real'], ['', 'clock +500 real'], ['', 'clock settle'], ['read', 'state sent'],
  // Another, never read; Disconnect, then Connect again at once: the close released the port, so the second open is
  // not refused as in use.
  ...send, ['', 'tap disconnect'], ['', 'clock settle'], ['', 'tap connect'], ['', 'clock settle'], ['', 'clock +500 real'], ['', 'clock settle'], ['again', 'state conn']];
const ops = steps.map(([, op]) => op);
const agent = spawn('bun', ['scripts/agent.mjs', host, '--json', '--app', 'bench', '--bind', 'serial', '--storage', `serial-pty-${Date.now()}`, '--reset-storage', ...ops],
  { cwd: root, env: { ...process.env, EXACT_DEVICE_FIXTURE: fixture }, stdio: ['ignore', 'pipe', 'pipe'] });
let stderr = '';
agent.stderr.on('data', (d) => { stderr = (stderr + d).slice(-3000); });
const replies = [];
let received = null;
// A host whose loop a write holds answers nothing more: the drive is given up after three minutes without a reply.
let limit = setTimeout(() => agent.kill(), 180000);
for await (const line of createInterface({ input: agent.stdout })) {
  if (!/^[[{"]|^(null|true|false|-?\d)/.test(line)) continue;
  replies.push(JSON.parse(line));
  clearTimeout(limit);
  limit = setTimeout(() => agent.kill(), 180000);
  if (replies.length - 1 === steps.findIndex(([l]) => l === 'unread')) {
    writer.stdin.write(`${SENT + 2}\n`);
    received = fromWriter.next().then((r) => r.value ?? '');
  }
}
clearTimeout(limit);
writer.kill();
const at = (label) => { const v = replies[steps.findIndex(([l]) => l === label)]; return v?.value ?? v; };
if (replies.length < ops.length) console.log(ops.map((o, i) => `${i} ${o.slice(0, 40)} → ${JSON.stringify(replies[i])?.slice(0, 200)}`).filter((_, i) => i >= replies.length - 3 && i <= replies.length).join('\n'), stderr);
const ports = at('ports') ?? [];
check('the injected pty is listed beside the Mac\'s own ports', JSON.stringify(ports).includes('Fixture port'), JSON.stringify(ports).slice(0, 300));
const log = at('log') ?? {}, conn = at('conn') ?? {};
const lines = log.lines ?? [];
const numbered = lines.map((l) => Number(/^line (\d{5}) /.exec(l.text)?.[1]));
check('the port opened through the module', conn.open === true, JSON.stringify(conn));
check(`all ${LINES} lines arrived (${LINES * WIDTH} bytes) while a write the other end did not read was waiting`, log.total === LINES, `${log.total} lines kept`);
check('the lines shown are the last, in order, whole', numbered.length > 0 && numbered.at(-1) === LINES - 1 && numbered.every((n, i) => i === 0 || n === numbered[i - 1] + 1) && lines.every((l) => l.text.length === WIDTH - 2), JSON.stringify(lines.slice(-2)));
check('the write had not answered while its bytes were unread', replies.length > ops.indexOf('state sent') && at('unread') == null,JSON.stringify(at('unread')));
const line = received ? await received : '';
check(`the other end read the line whole (${SENT} bytes and CR LF)`, line === `${'x'.repeat(SENT)}CRLF`, `${line.length} characters`);
check('the write answered once its bytes were read', at('read')?.sent === true, JSON.stringify(at('read')));
check('with a second write unread, Disconnect closed the port and Connect opened it again', replies.length === ops.length && at('again')?.open === true, JSON.stringify(at('again')));
console.log(`${failed()} failed`);
process.exit(failed() ? 1 : 0);
