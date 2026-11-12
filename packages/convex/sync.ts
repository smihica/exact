// The client half of Convex's sync protocol (LLP 1157 D3), with no I/O: it is
// told what happened (the socket opened, a frame arrived, the app asked) and
// returns the frames to send. It reads no clock, no randomness and no global,
// and imports nothing, so a data module can hold it (LLP 1027.000). The
// protocol is the one `convex@1.49.0` speaks (its `src/browser/sync/`: `protocol.ts`,
// `local_state.ts`, `remote_query_set.ts`, `request_manager.ts`), and each
// rule below names the file it follows.

/** Convex's value encoding, from the app's own `convex/values`: this module imports no package, so an app has one copy of `convex`. */
export interface Codec { convexToJson(value: any): any; jsonToConvex(json: any): any }
type Value = unknown;

/** The npm client version whose protocol this speaks; it is the socket URL's `/api/<version>/sync`. */
export const PROTOCOL = '1.49.0';

export type Frame = { type: string; [field: string]: unknown };
type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type Version = { querySet: number; identity: number; ts: string };
/** A query's or a request's outcome: Convex's value, or its error's words and `ConvexError` data. */
export type Outcome = { ok: true; value: Value } | { ok: false; message: string; data?: Value };

/** A Convex function's failure: `message` is the server's, `data` a `ConvexError`'s payload. */
export class ConvexFailure extends Error {
  data: Value | undefined;
  constructor(message: string, data?: Value) { super(message); this.name = 'ConvexFailure'; this.data = data; }
}

type Query = {
  id: number; udfPath: string; args: Json;
  /** The server's opaque cursor state for a paginated query, sent back on a reconnect (`local_state.ts`, `restart`). */
  journal: string | null | undefined;
  outcome: Outcome | undefined;
  /** The generation of asks that last asked for it (see `ask`). */
  asked: number;
  /** The server has been told of it on this connection. */
  sent: boolean;
  /** Settles when an outcome arrives or the socket ends; one promise, however many asks wait. */
  wait?: { promise: Promise<void>; settle: () => void };
};
type Request = {
  kind: 'Mutation' | 'Action';
  /** A mutation the server ran, held until a `Transition` reaches `ts`, so its caller reads its own write. */
  done?: { ts: string; value: Value };
  settle: (outcome: Outcome) => void;
};

const ZERO_TS = 'AAAAAAAAAAA=';
const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
/** A timestamp's eight bytes, most significant first: the wire's is a base64 little-endian u64 (`protocol.ts`, `u64ToLong`). */
function tsBytes(ts: string): number[] {
  const bytes: number[] = [];
  let bits = 0, held = 0;
  for (const c of ts) {
    const v = B64.indexOf(c);
    if (v < 0) continue;
    held = (held << 6) | v; bits += 6;
    if (bits >= 8) { bits -= 8; bytes.push((held >> bits) & 0xff); }
  }
  return bytes.slice(0, 8).reverse();
}
/** Whether timestamp `a` is at or before `b`. */
export function tsAtOrBefore(a: string, b: string): boolean {
  const x = tsBytes(a), y = tsBytes(b);
  for (let i = 0; i < 8; i++) if (x[i] !== y[i]) return x[i] < y[i];
  return true;
}

/** `messages:list` for `messages:list`, `messages.js:list` and a bare module's `default` (`udf_path_utils.ts`). */
export function canonicalPath(udfPath: string): string {
  const pieces = udfPath.split(':');
  const name = pieces.length === 1 ? 'default' : pieces[pieces.length - 1];
  let module = pieces.length === 1 ? pieces[0] : pieces.slice(0, -1).join(':');
  if (module.endsWith('.js')) module = module.slice(0, -3);
  return `${module}:${name}`;
}

/** A frame this client cannot follow. `fatal`: a new connection would meet it again. */
export class ProtocolError extends Error {
  fatal: boolean;
  constructor(message: string, fatal = false) { super(message); this.name = 'ProtocolError'; this.fatal = fatal; }
}

export class Sync {
  /** `connecting` until the socket opens, `open` until its first `Transition`, `ready` from then; `closed` once it has
   *  ended, or `failed` when it ended on a frame this client cannot follow, which a new connection would meet again. */
  status: 'connecting' | 'open' | 'ready' | 'closed' | 'failed' = 'connecting';
  /** Rises when an answer a query gave may have changed: a `Transition` applied, or the identity changed. */
  rev = 0;
  /** Why the socket last ended; "" while it is open. */
  error = '';
  /** The server's words when it refused the token; "" otherwise. */
  authError = '';

  private sessionId = '';
  private connectionCount = 0;
  private lastCloseReason: string | null = 'InitialConnect';
  private maxObservedTs: string | undefined;
  /** What the server's next `Transition` must start from (`remote_query_set.ts`). */
  private version: Version = { querySet: 0, identity: 0, ts: ZERO_TS };
  private querySetVersion = 0;
  private identityVersion = 0;
  private token: string | undefined;
  /** The identity the cached outcomes were computed for is not the one asked for: none is served until a `Transition` reaches `identityVersion`. */
  private identityStale = false;
  private queries = new Map<string, Query>();
  private byId = new Map<number, Query>();
  private nextQueryId = 0;
  private requests = new Map<number, Request>();
  private nextRequestId = 0;
  /** The newest `rev` an ask has carried, and the generation of asks before it. */
  private generation = -1;
  private chunks: string[] = [];
  private chunkId = '';

  constructor(private codec: Codec) {}

  private connected() { return this.status === 'open' || this.status === 'ready'; }

  /** A socket is being opened: `sessionId` names this client for as long as the module lives, so it is kept from the first. */
  connecting(sessionId: string) {
    if (!this.sessionId) this.sessionId = sessionId;
    this.status = 'connecting';
    this.error = '';
  }

  /** The socket opened: what the npm client sends then, in its order (`client.ts`, `onOpen`). The server
   *  starts every connection at version 0, so the token and the whole query set go again, and the next
   *  `Transition` is expected from 0. */
  opened(): Frame[] {
    this.status = 'open';
    this.version = { querySet: 0, identity: 0, ts: ZERO_TS };
    this.chunks = [];
    const frames: Frame[] = [{
      type: 'Connect', sessionId: this.sessionId, connectionCount: this.connectionCount, lastCloseReason: this.lastCloseReason,
      ...(this.maxObservedTs ? { maxObservedTimestamp: this.maxObservedTs } : {}),
      // The npm client's monotonic clock, for the server's skew estimate; a data module has none.
      clientTs: 0,
    }];
    this.connectionCount += 1;
    if (this.token !== undefined) {
      frames.push({ type: 'Authenticate', tokenType: 'User', value: this.token, baseVersion: 0 });
      this.identityVersion = 1;
      this.identityStale = true;
    } else {
      this.identityVersion = 0;
      this.identityStale = false;
    }
    const adds = [...this.queries.values()].map(q => (q.sent = true, this.add(q)));
    frames.push({ type: 'ModifyQuerySet', baseVersion: 0, newVersion: 1, modifications: adds });
    this.querySetVersion = 1;
    return frames;
  }

  /** The socket ended, with why; `fatal` when a new connection would end the same way. A query with no
   *  answer yet fails, and is asked again when one arrives. A request in flight ends here too: an answer
   *  cannot wait across the gap (natively one that awaits with nothing outstanding is refused), so, unlike
   *  the npm client (`request_manager.ts`, `restart`), this sends no mutation again on the next connection.
   *  One the server acknowledged succeeded; any other may have run. */
  closed(reason: string, fatal = false) {
    // The close that follows a fatal end is that end's, not a new one: `failed` and its reason stand.
    if (this.status === 'failed') return;
    this.status = fatal ? 'failed' : 'closed';
    this.error = reason;
    this.lastCloseReason = reason;
    for (const q of this.queries.values()) { q.sent = false; this.wake(q); }
    for (const request of this.requests.values())
      request.settle(request.done ? { ok: true, value: request.done.value } : { ok: false, message: `The connection to Convex closed before it answered (${reason}); the ${request.kind.toLowerCase()} may have run.` });
    this.requests.clear();
  }

  /** A frame from the server, taken into the state. One this client cannot follow throws a `ProtocolError`:
   *  the caller ends the socket with its words. */
  frame(text: string) {
    const message = JSON.parse(text) as Frame;
    if (this.chunks.length && message.type !== 'TransitionChunk') throw new ProtocolError(`Convex sent ${String(message.type)} inside a chunked Transition`);
    switch (message.type) {
      case 'Ping': return;
      case 'TransitionChunk': {
        // A large `Transition` in parts, in order (`web_socket_manager.ts`, `assembleTransition`).
        const { chunk, partNumber, totalParts, transitionId } = message as unknown as { chunk: string; partNumber: number; totalParts: number; transitionId: string };
        if (partNumber !== this.chunks.length || (partNumber > 0 && transitionId !== this.chunkId)) throw new ProtocolError(`Convex sent TransitionChunk ${partNumber} of ${transitionId}, expected ${this.chunks.length}${this.chunks.length ? ` of ${this.chunkId}` : ''}`);
        this.chunkId = transitionId;
        this.chunks.push(chunk);
        if (this.chunks.length < totalParts) return;
        const whole = this.chunks.join('');
        this.chunks = [];
        this.transition(JSON.parse(whole));
        return;
      }
      case 'Transition': this.transition(message as never); return;
      case 'MutationResponse':
      case 'ActionResponse': this.response(message as never); return;
      case 'AuthError': {
        // The server closes the socket after it. The token is dropped, so the next connection is not refused
        // for it too; the app gets a fresh one and calls `setAuth`.
        this.authError = String(message.error ?? 'Convex refused the token');
        this.token = undefined;
        return;
      }
      case 'FatalError': throw new ProtocolError(`Convex: ${String(message.error)}`, true);
      default: throw new ProtocolError(`Convex sent ${String(message.type)}, which protocol ${PROTOCOL} does not have`, true);
    }
  }

  private transition(t: { startVersion: Version; endVersion: Version; modifications: Array<Record<string, unknown>> }) {
    const { startVersion: start, endVersion: end } = t, at = this.version;
    if (start.querySet !== at.querySet || start.identity !== at.identity || start.ts !== at.ts)
      throw new ProtocolError(`Convex sent a Transition from ${start.querySet}:${start.identity}:${start.ts}, expected ${at.querySet}:${at.identity}:${at.ts}`);
    for (const m of t.modifications) {
      const q = this.byId.get(m.queryId as number);
      // (A query removed here while its result was on the way.)
      if (!q) continue;
      if (m.type === 'QueryUpdated') q.outcome = { ok: true, value: this.codec.jsonToConvex(m.value as never) };
      else if (m.type === 'QueryFailed') q.outcome = { ok: false, message: String(m.errorMessage), ...(m.errorData === undefined ? {} : { data: this.codec.jsonToConvex(m.errorData as never) }) };
      else continue;
      if (m.journal !== undefined) q.journal = m.journal as string | null;
      this.wake(q);
    }
    this.version = end;
    this.status = 'ready';
    this.observe(end.ts);
    if (this.identityStale && end.identity >= this.identityVersion) {
      // The identity is confirmed: the answers it left unnamed are this person's too, and their asks may go on.
      this.identityStale = false;
      for (const q of this.queries.values()) this.wake(q);
    }
    for (const [id, request] of this.requests) {
      if (!request.done || !tsAtOrBefore(request.done.ts, end.ts)) continue;
      this.requests.delete(id);
      request.settle({ ok: true, value: request.done.value });
    }
    this.rev += 1;
  }

  private response(r: { type: string; requestId: number; success: boolean; result: Json; ts?: string; errorData?: Json }) {
    const request = this.requests.get(r.requestId);
    if (!request || request.done) return;
    if (!r.success) {
      this.requests.delete(r.requestId);
      request.settle({ ok: false, message: String(r.result), ...(r.errorData === undefined ? {} : { data: this.codec.jsonToConvex(r.errorData as never) }) });
      return;
    }
    const value = this.codec.jsonToConvex(r.result as never);
    if (r.type === 'ActionResponse') { this.requests.delete(r.requestId); request.settle({ ok: true, value }); return; }
    // Held until the queries show it (`request_manager.ts`, `onResponse`), or settled now when they already do.
    this.observe(r.ts!);
    if (tsAtOrBefore(r.ts!, this.version.ts)) { this.requests.delete(r.requestId); request.settle({ ok: true, value }); }
    else request.done = { ts: r.ts!, value };
  }

  private observe(ts: string) {
    if (this.maxObservedTs === undefined || tsAtOrBefore(this.maxObservedTs, ts)) this.maxObservedTs = ts;
  }

  private add(q: Query): Frame {
    return { type: 'Add', queryId: q.id, udfPath: q.udfPath, args: [q.args], ...(q.journal === undefined ? {} : { journal: q.journal }) };
  }

  private wake(q: Query) {
    const wait = q.wait;
    q.wait = undefined;
    wait?.settle();
  }

  /** The app asks for a query's answer at `rev`. `read` gives its outcome, or nothing when there is none
   *  yet: the caller then waits on `wait`, which settles when one arrives or the socket ends. `frames`
   *  tell the server of a query new to it, and remove the ones nobody asks for any more.
   *
   *  A query resource takes `rev` as an argument, so every live one is asked again at each `rev`
   *  (LLP 1157 D4). The asks at one `rev` are a generation; when the first ask of a newer one arrives,
   *  a query the generation before it did not ask for has no resource left, and is removed. */
  ask(rev: number, udfPath: string, args: Record<string, Value>): { read: () => Outcome | undefined; wait: Promise<void>; frames: Frame[] } {
    const mods: Frame[] = [];
    if (rev > this.generation) {
      for (const [token, q] of this.queries) {
        if (q.asked >= this.generation) continue;
        this.queries.delete(token);
        this.byId.delete(q.id);
        this.wake(q);
        if (q.sent && this.connected()) mods.push({ type: 'Remove', queryId: q.id });
      }
      this.generation = rev;
    }
    const path = canonicalPath(udfPath), json = this.codec.convexToJson(args as never) as Json;
    const token = JSON.stringify({ udfPath: path, args: json });
    let q = this.queries.get(token);
    if (!q) {
      q = { id: this.nextQueryId++, udfPath: path, args: json, journal: undefined, outcome: undefined, asked: this.generation, sent: false };
      this.queries.set(token, q);
      this.byId.set(q.id, q);
    }
    q.asked = this.generation;
    if (!q.sent && this.connected()) { q.sent = true; mods.push(this.add(q)); }
    const frames: Frame[] = [];
    if (mods.length) {
      frames.push({ type: 'ModifyQuerySet', baseVersion: this.querySetVersion, newVersion: this.querySetVersion + 1, modifications: mods });
      this.querySetVersion += 1;
    }
    const waiting = q, read = () => (waiting.outcome !== undefined && !this.identityStale ? waiting.outcome : undefined);
    if (read() === undefined && !q.wait) {
      let settle = () => {};
      q.wait = { promise: new Promise<void>(resolve => { settle = resolve; }), settle };
    }
    return { read, frames, wait: q.wait?.promise ?? Promise.resolve() };
  }

  /** A mutation or an action, while the socket is open: the frame to send and its outcome. A mutation's
   *  outcome waits for the `Transition` that shows its write. */
  request(kind: 'Mutation' | 'Action', udfPath: string, args: Record<string, Value>): { frames: Frame[]; outcome: Promise<Outcome> } {
    const requestId = this.nextRequestId++;
    const message: Frame = { type: kind, requestId, udfPath: canonicalPath(udfPath), args: [this.codec.convexToJson(args as never)] };
    const outcome = new Promise<Outcome>(settle => this.requests.set(requestId, { kind, settle }));
    return { frames: [message], outcome };
  }

  /** The person's token changed (`undefined`: signed out). Answers computed for the identity before are
   *  kept, since the server's next `Transition` names only the ones that changed, and none is served until
   *  that `Transition` arrives and raises `rev` (`local_state.ts`, `setAuth`). `deliver` sends the frame. */
  setAuth(token: string | undefined, deliver: (frames: Frame[]) => void = () => {}): Frame[] {
    if (token === this.token) return [];
    const baseVersion = this.identityVersion;
    const frames: Frame[] = !this.connected() ? [] : [token === undefined ? { type: 'Authenticate', tokenType: 'None', baseVersion } : { type: 'Authenticate', tokenType: 'User', value: token, baseVersion }];
    // Sent before anything here changes: a turn that may not send throws, and the identity is still the one Convex has.
    deliver(frames);
    this.token = token;
    this.authError = '';
    this.identityStale = true;
    this.identityVersion += frames.length;
    return frames;
  }

  /** For `state` and tests: the queries the server is asked for, and the requests in flight. */
  inspect() {
    return { queries: [...this.queries.values()].map(q => ({ id: q.id, udfPath: q.udfPath, sent: q.sent, answered: q.outcome !== undefined })), requests: this.requests.size };
  }
}
