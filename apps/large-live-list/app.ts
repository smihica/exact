import type { Answer, Database, Result, Sources, Storage, StreamSocket } from './app.contract.d.ts';

const FIXTURE = 'wss://example.invalid';
const SOCKET = import.meta.env.MESSAGE_SOCKET ?? 'wss://example.invalid';
const SOCKET_DB = 'app:/data/messages.db';
const FIXTURE_DB = 'app:/data/fixture.db';
const DB = SOCKET === FIXTURE ? FIXTURE_DB : SOCKET_DB;
export const grants = `sqlite.open ${SOCKET_DB}\nsqlite.open ${FIXTURE_DB}\nnet.websocket ${SOCKET}`;

const progress = 'CREATE TABLE IF NOT EXISTS socket_progress (conversation TEXT PRIMARY KEY, sequence INTEGER NOT NULL)';

const schema = [
  { sql: 'CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, last_order INTEGER NOT NULL, unread INTEGER NOT NULL, queued INTEGER NOT NULL)' },
  { sql: 'CREATE TABLE IF NOT EXISTS messages (conversation TEXT NOT NULL, sort_order INTEGER NOT NULL, id TEXT NOT NULL, body TEXT NOT NULL, PRIMARY KEY (conversation, id))' },
  { sql: 'CREATE INDEX IF NOT EXISTS message_order ON messages (conversation, sort_order, id)' },
  { sql: 'CREATE TABLE IF NOT EXISTS outbox (position INTEGER PRIMARY KEY, conversation TEXT NOT NULL, id TEXT NOT NULL, body TEXT NOT NULL, UNIQUE (conversation, id))' },
  { sql: 'CREATE INDEX IF NOT EXISTS outbox_order ON outbox (conversation, position)' },
];

let storageTail: Promise<unknown> = Promise.resolve();
function withDatabase<T>(storage: Storage, work: (db: Database) => Promise<T>): Promise<T> {
  const result = storageTail.then(async () => {
    const db = await storage.sqlite.open(DB);
    try { return await work(db); } finally { await db.close(); }
  });
  storageTail = result.catch(() => {});
  return result;
}

async function hasSchema(db: Database) {
  const tables = await db.query("SELECT 1 FROM sqlite_schema WHERE name = 'outbox_order'");
  return tables.rows.length > 0;
}

async function seed(db: Database, conversation: string) {
  if (!['1000', '10000', '100000'].includes(conversation)) throw new Error('Unknown conversation');
  if (!await hasSchema(db)) await db.transaction(schema);
  const existing = await db.query('SELECT 1 FROM conversations WHERE id = ?', [conversation]);
  if (existing.rows.length) return;
  await db.transaction([
    { sql: 'INSERT INTO conversations VALUES (?, ?, 0, 0)', params: [conversation, Number(conversation)] },
    { sql: `WITH RECURSIVE numbers(n) AS (VALUES (1) UNION ALL SELECT n + 1 FROM numbers WHERE n < ?)
            INSERT INTO messages SELECT ?, n, 'seed-' || n, 'Message ' || n FROM numbers`,
      params: [Number(conversation), conversation] },
  ]);
}

async function windowAt(db: Database, conversation: string, anchor: string): Promise<Result<'loadWindow'>> {
  const empty = { rows: [], earlier: '', later: '', conversation, anchor, total: 0, unread: 0, queued: 0 };
  if (!conversation || !await hasSchema(db)) return empty;
  const { rows: [counts] } = await db.query(
    'SELECT last_order, unread, queued FROM conversations WHERE id = ?', [conversation]);
  if (!counts) return empty;

  const [order, id] = anchor.split('/');
  const older = anchor
    ? await db.query(`SELECT sort_order, id, body FROM messages WHERE conversation = ?
                      AND (sort_order, id) <= (?, ?) ORDER BY sort_order DESC, id DESC LIMIT 101`,
      [conversation, Number(order), id])
    : await db.query(`SELECT sort_order, id, body FROM messages WHERE conversation = ?
                      ORDER BY sort_order DESC, id DESC LIMIT 201`, [conversation]);
  const newer = anchor
    ? await db.query(`SELECT sort_order, id, body FROM messages WHERE conversation = ?
                      AND (sort_order, id) > (?, ?) ORDER BY sort_order, id LIMIT 101`,
      [conversation, Number(order), id])
    : { rows: [] };
  const size = anchor ? 100 : 200;
  const rows = older.rows.slice(0, size).reverse().concat(newer.rows.slice(0, 100));
  const [first, last] = [rows[0], rows.at(-1)];
  return {
    rows: rows.map(([, id, body]) => ({ id: String(id), body: String(body) })),
    earlier: older.rows.length > size && first ? `${first[0]}/${first[1]}` : '',
    later: newer.rows.length > 100 && last ? `${last[0]}/${last[1]}` : '',
    conversation, anchor, total: Number(counts[0]), unread: Number(counts[1]), queued: Number(counts[2]),
  };
}

function insertMessage(conversation: string, id: string, body: string, unread: number, order?: number) {
  return [
    { sql: `INSERT INTO messages SELECT id, coalesce(?, last_order + 1), ?, ? FROM conversations WHERE id = ?
            ON CONFLICT (conversation, id) DO NOTHING`, params: [order ?? null, id, body, conversation] },
    { sql: 'UPDATE conversations SET last_order = last_order + 1, unread = unread + ? WHERE id = ? AND changes() = 1',
      params: [unread, conversation] },
  ];
}

async function queue(db: Database, conversation: string, body: string) {
  if (!body || body.length > 512) throw new Error('Write a message of at most 512 characters');
  const [added] = await db.transaction([
    { sql: `INSERT INTO outbox (conversation, id, body) SELECT id, 'send-' || lower(hex(randomblob(16))), ?
            FROM conversations WHERE id = ? AND queued < 100`, params: [body, conversation] },
    { sql: 'UPDATE conversations SET queued = queued + 1 WHERE id = ? AND changes() = 1', params: [conversation] },
  ]);
  if (!added.changes) throw new Error('The outbox is full');
}

function retireSend(conversation: string, id: string) {
  return [
    { sql: 'DELETE FROM outbox WHERE conversation = ? AND id = ?', params: [conversation, id] },
    { sql: 'UPDATE conversations SET queued = queued - 1 WHERE id = ? AND changes() = 1', params: [conversation] },
  ];
}

async function oldestSend(db: Database, conversation: string) {
  const { rows: [row] } = await db.query(
    'SELECT id, body FROM outbox WHERE conversation = ? ORDER BY position LIMIT 1', [conversation]);
  return row ? { id: String(row[0]), body: String(row[1]) } : undefined;
}

async function replayFixture(db: Database, conversation: string) {
  for (;;) {
    const next = await oldestSend(db, conversation);
    if (!next) return;
    await db.transaction([
      ...insertMessage(conversation, next.id, next.body, 0),
      ...retireSend(conversation, next.id),
    ]);
  }
}

type Frame = { type: 'message' | 'sent' | 'ack'; sequence: number; id: string; body: string; sender: string };
type Connection = {
  attempt: number;
  serial: number;
  inbox: Frame[];
  socket?: StreamSocket;
  sender?: string;
  start?: number;
  sequence?: number;
  gap?: boolean;
  sending?: string;
};
let connection: Connection | undefined;

function parseFrame(data: unknown): Frame | undefined {
  if (typeof data !== 'string' || data.length > 4096) return;
  let frame;
  try { frame = JSON.parse(data); } catch { return; }
  if ((frame?.type === 'message' || frame?.type === 'sent' || frame?.type === 'ack') && Number.isSafeInteger(frame.sequence)
      && typeof frame.id === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(frame.id)
      && typeof frame.sender === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(frame.sender)
      && typeof frame.body === 'string' && frame.body.length <= 512)
    return frame;
}

async function resumeState(db: Database, conversation: string) {
  await db.transaction([
    { sql: progress },
    { sql: 'CREATE TABLE IF NOT EXISTS socket_client (slot INTEGER PRIMARY KEY, id TEXT NOT NULL)' },
    { sql: 'INSERT OR IGNORE INTO socket_client VALUES (1, lower(hex(randomblob(16))))' },
  ]);
  const { rows: [[sender, sequence]] } = await db.query(
    'SELECT id, (SELECT sequence FROM socket_progress WHERE conversation = ?) FROM socket_client WHERE slot = 1', [conversation]);
  // Server sequences follow the numbered seed rows.
  return { sender: String(sender), after: Number(sequence ?? conversation) };
}

async function commitFrames(db: Database, live: Connection, conversation: string, after: number) {
  const frames = live.inbox.slice(0, 512);
  let sequence = after;
  let consumed = 0;
  let acknowledged = false;
  const statements: Parameters<Database['transaction']>[0] = [];
  for (const frame of frames) {
    if (frame.sequence > sequence + 1) break;
    consumed++;
    if (frame.sequence === sequence + 1) {
      statements.push(...insertMessage(conversation, frame.id, frame.body, frame.sender === live.sender ? 0 : 1, frame.sequence));
      sequence = frame.sequence;
    }
    if (frame.type === 'ack') {
      statements.push(...retireSend(conversation, frame.id));
      if (live.sending === frame.id) acknowledged = true;
    }
  }
  if (sequence !== after)
    statements.push({ sql: `INSERT INTO socket_progress VALUES (?, ?)
      ON CONFLICT (conversation) DO UPDATE SET sequence = excluded.sequence`, params: [conversation, sequence] });
  const results = await db.transaction(statements);
  if (sequence !== after) results.pop();
  live.sequence = sequence;
  live.gap = consumed < frames.length;
  live.inbox.splice(0, live.gap ? live.inbox.length : consumed);
  if (acknowledged) live.sending = undefined;
  return results.some(result => result.changes > 0);
}

const sources: Sources = {
  socketMode: () => SOCKET === FIXTURE ? 'fixture' : 'socket',
  loadWindow: ([conversation, cursor], _store, storage) =>
    withDatabase(storage, db => windowAt(db, conversation, cursor)),
  change: ([op, conversation, body], _store, storage) =>
    withDatabase(storage, async db => {
      switch (op) {
        case 'seed': await seed(db, conversation); break;
        case 'queue': await queue(db, conversation, body); break;
        case 'replay':
          if (SOCKET !== FIXTURE) throw new Error('Queued sends require a server acknowledgement');
          await replayFixture(db, conversation);
          break;
        case 'arrive': {
          const { rows: [[last]] } = await db.query('SELECT last_order FROM conversations WHERE id = ?', [conversation]);
          const next = Number(last) + 1;
          await db.transaction(insertMessage(conversation, `arrival-${next}`, `Arrival ${next}`, 1));
          break;
        }
        case 'read': await db.execute('UPDATE conversations SET unread = 0 WHERE id = ?', [conversation]); break;
        default: throw new Error('Unknown change');
      }
      return { op, conversation };
    }),
  events: ([_conversation, attempt]) => {
    const live: Connection = { attempt, serial: 0, inbox: [] };
    connection = live;
    return fetch(SOCKET, {
      exactStream(event, socket) {
        if (event.type === 'open') live.socket = socket;
        if (event.type === 'close' || event.type === 'error') live.socket = undefined;
        if (event.type === 'error') throw new Error(event.message);
        if (event.type === 'message') {
          const frame = parseFrame(event.data);
          if (!frame) {
            socket?.close(4000, 'Invalid frame');
            return { status: 'Socket frame refused', serial: ++live.serial };
          }
          if (live === connection && !live.gap) live.inbox.push(frame);
        }
        return { status: event.type, serial: ++live.serial };
      },
    });
  },
  syncSocket: async ([conversation, attempt, failed], _store, storage) => {
    const live = connection;
    if (live?.attempt !== attempt) return { attempt, changed: false, more: false, recover: false, advanced: false };
    if (!failed && live.sequence === undefined && live.socket?.readyState === 1) {
      const { sender, after } = await withDatabase(storage, db => resumeState(db, conversation));
      // A socket command must run in this source's turn, outside the shared storage queue.
      if (live === connection && live.socket?.readyState === 1) {
        live.socket.send(JSON.stringify({ type: 'resume', conversation, after, sender }));
        live.sender = sender;
        live.start = after;
        live.sequence = after;
      }
    }
    const after = live.sequence;
    const changed = live.inbox.length > 0 && after !== undefined
      && await withDatabase(storage, db => commitFrames(db, live, conversation, after));
    return {
      attempt, changed, more: live.inbox.length > 0,
      recover: failed || !!live.gap, advanced: live.sequence !== live.start,
    };
  },
  sendSocket: async ([conversation, attempt], _store, storage) => {
    const live = connection;
    if (live?.attempt !== attempt || live.gap || live.sending || live.sequence === undefined
        || live.socket?.readyState !== 1) return false;
    const next = await withDatabase(storage, db => oldestSend(db, conversation));
    if (!next || live !== connection || live.socket?.readyState !== 1) return false;
    live.socket.send(JSON.stringify({ type: 'send', conversation, ...next }));
    live.sending = next.id;
    return true;
  },
};

export const answer: Answer = (name, args, store, storage, native) =>
  sources[name](args, store, storage, native);
