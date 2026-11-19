import assert from 'node:assert/strict';
import { afterAll, beforeAll, test } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { open, runTests } from '../../scripts/agent.mjs';
import { chromium, driveStore, removeDriveStore } from '../../scripts/agent-launch.mjs';
import { resolveApp } from '../../scripts/app.mjs';
import { need } from '../../scripts/coverage.mjs';
import { serveMessages } from './socket.mjs';

const app = 'large-live-list';
const root = resolve(import.meta.dir, '../..');
const storage = `large-live-list-probe-${process.pid}`;
const peerStorage = `${storage}-peer`;
const webTest = need('chrome', chromium().unavailable);
const webBuilds = new Map();
let fixture, outputs;

beforeAll(() => {
  mkdirSync(resolve(root, 'target'), { recursive: true });
  outputs = mkdtempSync(resolve(root, 'target/large-live-list-tests-'));
  fixture = serveMessages({ test: true });
});
afterAll(async () => {
  fixture?.close();
  for (const name of [storage, peerStorage]) await removeDriveStore(driveStore(resolveApp(app), name), name);
  if (outputs) rmSync(outputs, { recursive: true, force: true });
});

async function build(host, mode) {
  const webDist = resolve(outputs, mode);
  const env = { ...process.env, EXACT_FEEDBACK: '0' };
  if (mode === 'socket') env.MESSAGE_SOCKET = fixture.url;
  else delete env.MESSAGE_SOCKET;
  if (host === 'web') env.EXACT_WEB_DIST = webDist;
  const child = Bun.spawn([process.execPath, host === 'web' ? 'host/web/build.mjs' : 'host/apple/build.mjs', app], {
    cwd: root, env, stdout: 'inherit', stderr: 'inherit',
  });
  assert.equal(await child.exited, 0, `${host} ${mode} build`);
  return { webDist, env: { EXACT_BACKEND_ORIGIN: fixture.url.replace('ws:', 'http:') } };
}

function webBuild(mode) {
  if (!webBuilds.has(mode)) webBuilds.set(mode, build('web', mode));
  return webBuilds.get(mode);
}

async function withMode(mode, work) {
  const previous = process.env.MESSAGE_SOCKET;
  if (mode === 'socket') process.env.MESSAGE_SOCKET = fixture.url;
  else delete process.env.MESSAGE_SOCKET;
  try { return await work(); }
  finally {
    if (previous === undefined) delete process.env.MESSAGE_SOCKET;
    else process.env.MESSAGE_SOCKET = previous;
  }
}

async function request(action, fields = {}) {
  const response = await fetch(fixture.url.replace('ws:', 'http:') + '/control', {
    method: 'POST', body: JSON.stringify({ action, ...fields }),
    signal: AbortSignal.timeout(10000),
  });
  assert.ok(response.ok, `${action}: ${response.status} ${await response.text()}`);
}

async function drive(mode, name, fresh = false) {
  const { webDist, env } = await webBuild(mode);
  return withMode(mode, () => open({
    host: 'web', app, storage: name, size: [760, 700], webDist,
    env: { ...env, EXACT_AGENT_STORAGE_FRESH: fresh ? '1' : '0' },
  }));
}

async function prepare(session) {
  await session.tap('prepare-1000');
  await session.clock('data');
  await session.tap('open');
  await session.clock('settle');
}

async function connect(session) {
  await session.tap('connect');
  await session.clock('until state feed.status == "open" within 10000');
  await session.clock('data');
  await request('expect-resume', { count: 1000 });
}

async function transcript(work, join = true) {
  await request('reset');
  const session = await drive('socket', storage, true);
  try {
    await prepare(session);
    if (join) await connect(session);
    await work(session);
  } finally {
    await session.close();
  }
}

function interceptSQLite({ name, sql, fail }) {
  const post = Worker.prototype.postMessage;
  const reached = Promise.withResolvers();
  let release;
  Worker.prototype.postMessage = function (message, ...options) {
    const command = message.op === 'transaction'
      ? message.args[1][0]?.sql : message.args?.[1];
    if (message.op !== name || !command?.startsWith(sql)) return post.call(this, message, ...options);
    Worker.prototype.postMessage = post;
    reached.resolve();
    if (fail) {
      const { client, id } = message;
      queueMicrotask(() => this.dispatchEvent(new MessageEvent('message', {
        data: { client, id, error: { message: 'Test refused SQLite lookup' } },
      })));
    } else {
      release = () => post.call(this, message, ...options);
    }
  };
  globalThis.sqliteProbe = { reached: reached.promise, release: () => release() };
}

const arm = (session, name, sql, fail = false) =>
  session.carrier.evaluate(`(${interceptSQLite})(${JSON.stringify({ name, sql, fail })})`);
const reached = session => session.carrier.evaluate('sqliteProbe.reached', 10000);
const release = session => session.carrier.evaluate('sqliteProbe.release()');

webTest('a held save preserves edits including a return to the submitted text', () => transcript(async session => {
  for (const [index, draft] of ['Two', 'One'].entries()) {
    await session.type('draft', 'One');
    await arm(session, 'transaction', 'INSERT INTO outbox');
    await session.tap('send');
    await reached(session);
    await session.type('draft', 'Changed during save');
    await session.type('draft', draft);
    await release(session);
    await session.clock('data');
    const { slots, resources } = await session.state();
    assert.equal(slots.draft, draft);
    assert.equal(resources.window.queued, index + 1);
  }
}, false), 900000);

webTest('a page edge waits for the revision it reached', () => transcript(async session => {
  await arm(session, 'query', 'SELECT last_order, unread, queued');
  await request('arrive', { id: 'during-refresh', body: 'Arrival during refresh' });
  await session.clock('until state revision == 2 within 10000');
  await reached(session);
  await session.tap('transcript', { wheel: [0, -10000] });
  await session.clock('until state edge == "earlier" within 10000');
  assert.equal((await session.state()).slots.cursor, '');
  await release(session);
  await session.clock('settle');
  const { slots, resources } = await session.state();
  assert.equal(slots.cursor, '802/seed-802');
  assert.equal(resources.window.rows[99].id, 'seed-802');
}), 900000);

webTest('an edge on the visible tail cannot page the arriving older window', () => transcript(async session => {
  await arm(session, 'query', 'SELECT last_order, unread, queued');
  await session.tap('transcript', { wheel: [0, -10000] });
  await reached(session);
  await session.tap('transcript', { wheel: [0, 10000] });
  await session.clock('until state edge == "later" within 10000');
  await release(session);
  await session.clock('settle');
  const { slots, resources } = await session.state();
  assert.equal(slots.cursor, '801/seed-801');
  assert.equal(resources.window.rows[199].id, 'seed-901');
}, false), 900000);

webTest('an outbound lookup failure cannot hide a committed arrival', () => transcript(async session => {
  await arm(session, 'query', 'SELECT id, body FROM outbox', true);
  await request('arrive', { id: 'before-lookup-failure', body: 'Committed arrival' });
  await session.clock('until state revision == 2 within 10000');
  await session.clock('data');
  await reached(session);
  assert.ok(await session.find('retry-sync'));
  const { slots, resources } = await session.state();
  assert.equal(slots.revision, 2);
  assert.equal(resources.window.total, 1001);
  assert.equal(resources.window.unread, 1);
  assert.equal(resources.window.rows[199].id, 'before-lookup-failure');
}), 900000);

webTest('a burst above the receive bound resumes every row in server order', () => transcript(async session => {
  await session.carrier.evaluate(`(() => {
    const Socket = globalThis.WebSocket;
    globalThis.WebSocket = class extends Socket {
      set onmessage(handler) {
        const held = [];
        super.onmessage = event => {
          held.push(event);
          if (held.length === 300) {
            globalThis.WebSocket = Socket;
            for (const frame of held) handler(frame);
          }
        };
      }
    };
  })()`);
  await connect(session);
  await request('burst', { count: 300 });
  await session.clock('until state recovery == "resume" within 10000');
  assert.ok(await session.find('socket-failed'));
  await session.clock('+1');
  await session.clock('until state window.total == 1300 within 10000');
  assert.equal((await session.state()).slots.attempt, 2);
  await request('expect-resume', { count: 1256 });
  const tail = (await session.state()).resources.window.rows;
  await session.tap('transcript', { wheel: [0, -10000] });
  await session.clock('settle');
  await session.tap('transcript', { wheel: [0, -10000] });
  await session.clock('settle');
  const earlier = (await session.state()).resources.window.rows;
  const head = earlier.filter(row => row.id.startsWith('burst-')).slice(0, 100);
  const ids = [...head, ...tail].map(row => row.id);
  assert.deepEqual(ids, Array.from({ length: 300 }, (_, index) => `burst-${index + 1}`));
}, false), 900000);

webTest('an automatic attempt without progress stops until Retry', () => transcript(async session => {
  await request('gap', { count: 1 });
  await session.clock('until state recovery == "resume" within 10000');
  await session.clock('+1');
  await session.clock('until state recovery == "stop" within 10000');
  await session.clock('+1');
  assert.equal((await session.state()).slots.attempt, 2);
  assert.ok(await session.find('socket-failed'));
  await session.tap('retry-sync');
  await session.clock('until state window.total == 1003 within 10000');
  assert.equal((await session.state()).slots.attempt, 3);
}), 900000);

async function send(session, body) {
  await session.type('draft', body);
  await session.tap('send');
}

webTest('sender identity sets unread for live and replayed messages in either acknowledgement order', () => transcript(async sender => {
  const peer = await drive('socket', peerStorage, true);
  try {
    await prepare(peer);
    await connect(peer);
    await request('echo-before-ack');
    await send(sender, 'Echo first');
    await sender.clock('until state window.total == 1001 within 10000');
    await sender.clock('until state window.queued == 0 within 10000');
    await peer.clock('until state window.total == 1001 within 10000');
    const first = [(await sender.state()).resources.window.unread, (await peer.state()).resources.window.unread];
    await peer.tap('disconnect');
    await send(sender, 'Acknowledgement first');
    await sender.clock('until state window.total == 1002 within 10000');
    await request('echo', { body: 'Acknowledgement first' });
    await sender.clock('until state feed.serial == 5 within 10000');
    await sender.clock('data');
    await peer.tap('connect');
    await peer.clock('until state window.total == 1002 within 10000');
    assert.deepEqual([first, [(await sender.state()).resources.window.unread, (await peer.state()).resources.window.unread]], [[0, 1], [0, 2]]);
  } finally {
    await peer.close();
  }
}), 900000);

webTest('a fixture launch leaves the socket outbox for the server', async () => {
  await transcript(async session => {
    await send(session, 'Socket only');
    await session.clock('until state window.queued == 1 within 10000');
  }, false);
  const local = await drive('fixture', storage);
  try {
    await local.clock('settle');
    await local.tap('close');
    await prepare(local);
    assert.equal((await local.state()).resources.window.queued, 0);
  } finally {
    await local.close();
  }
  const socket = await drive('socket', storage);
  try {
    await socket.clock('data');
    await socket.tap('connect');
    await socket.clock('until state window.total == 1001 within 10000');
    await request('expect-sends', { body: 'Socket only', count: 1 });
  } finally {
    await socket.close();
  }
}, 900000);

// This app has no host crates. The hourly lane discovers this Bun file and
// runs the authored suites here after each host's matching build.
for (const host of ['web', ...(process.platform === 'darwin' ? ['macos'] : [])]) {
  for (const mode of ['fixture', 'socket']) {
    (host === 'web' ? webTest : test)(`${host} ${mode} Contract suite`, async () => {
      const { webDist, env } = host === 'web' ? await webBuild(mode) : await build(host, mode);
      const file = resolve(import.meta.dir, mode === 'fixture' ? 'app.test.contract' : 'socket.test.contract');
      const result = await withMode(mode, () => runTests({ host, app, file, webDist, env, size: [760, 700],
        atResult: row => console.log(`${host} ${mode}: ${row.name}${row.failures.length ? '\n' + row.failures.join('\n') : ' passed'}`),
      }));
      assert.equal(result.failed, 0);
      assert.ok(result.passed > 0);
    }, 900000);
  }
}
