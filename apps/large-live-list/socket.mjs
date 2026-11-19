const conversations = new Set(['1000', '10000', '100000']);
const validId = id => typeof id === 'string' && /^[a-zA-Z0-9_-]{1,80}$/.test(id);
const validBody = body => typeof body === 'string' && body.length > 0 && body.length <= 512;

export function serveMessages({ port = 0, greet = false, test = false } = {}) {
  const clients = new Map();
  const threads = new Map();
  let dropAck = false;
  let echoBeforeAck = false;
  const resumes = new EventTarget();

  function thread(conversation) {
    if (!threads.has(conversation))
      threads.set(conversation, { log: [], byId: new Map(), sends: [] });
    return threads.get(conversation);
  }

  function retain(conversation, type, id, body, sender = 'server') {
    const { log, byId } = thread(conversation);
    if (byId.has(id)) return byId.get(id);
    const frame = { type, id, body, sender, sequence: Number(conversation) + log.length + 1 };
    log.push(frame);
    byId.set(id, frame);
    return frame;
  }

  function broadcast(conversation, frame, except) {
    for (const [socket, joined] of clients) {
      if (joined?.conversation === conversation && socket !== except) socket.send(JSON.stringify(frame));
    }
  }

  function arrive(conversation, id, body) {
    broadcast(conversation, retain(conversation, 'message', id, body));
  }

  function disconnect() {
    for (const socket of clients.keys()) socket.close(1012, 'Fixture disconnected');
    clients.clear();
  }

  async function control(request) {
    const { action, conversation = '1000', id, body, count } = await request.json();
    if (!conversations.has(conversation)) return new Response('Unknown conversation', { status: 400 });
    const history = thread(conversation);
    if (action === 'reset') {
      disconnect();
      threads.clear();
      dropAck = false;
      echoBeforeAck = false;
    } else if (action === 'disconnect') {
      disconnect();
    } else if (action === 'arrive' && validId(id) && validBody(body)) {
      arrive(conversation, id, body);
    } else if (action === 'burst' && Number.isInteger(count) && count > 0 && count <= 1000) {
      for (let n = 1; n <= count; n++) arrive(conversation, `burst-${n}`, `Burst ${n}`);
    } else if (action === 'gap') {
      arrive(conversation, 'prefix', 'Contiguous prefix');
      retain(conversation, 'message', 'withheld', 'Withheld from the live socket');
      arrive(conversation, 'after-gap', 'After the gap');
      history.holdReplay = count;
    } else if (action === 'echo-before-ack') {
      echoBeforeAck = true;
    } else if (action === 'drop-ack') {
      dropAck = true;
    } else if (action === 'expect-resume') {
      if (history.after !== count) {
        await new Promise(resolve => {
          const deadline = AbortSignal.timeout(10000);
          deadline.addEventListener('abort', resolve, { once: true });
          resumes.addEventListener(conversation, () => {
            if (history.after === count) resolve();
          }, { signal: deadline });
        });
      }
      return new Response(String(history.after), { status: history.after === count ? 200 : 409 });
    } else if (action === 'expect-sends') {
      const sends = history.sends.filter(send => send.body === body);
      return new Response(String(sends.length), {
        status: sends.length === count && new Set(sends.map(send => send.id)).size === Math.min(count, 1) ? 200 : 409,
      });
    } else if (action === 'echo') {
      const frame = history.log.find(frame => frame.type === 'sent' && frame.body === body);
      if (!frame) return new Response('Send not found', { status: 404 });
      broadcast(conversation, { ...frame, type: 'message' });
    } else {
      return new Response('Unknown control', { status: 400 });
    }
    return new Response(null, { status: 204 });
  }

  const server = Bun.serve({
    hostname: '127.0.0.1', port,
    fetch(request, server) {
      const { pathname } = new URL(request.url);
      if (test && pathname === '/control' && request.method === 'POST') return control(request);
      if (pathname === '/' && server.upgrade(request)) return;
      return new Response('WebSocket upgrade required', { status: 426 });
    },
    websocket: {
      maxPayloadLength: 8192,
      closeOnBackpressureLimit: true,
      open(socket) { clients.set(socket, undefined); },
      message(socket, text) {
        if (typeof text !== 'string') return;
        let message;
        try { message = JSON.parse(text); } catch { return; }
        const { conversation } = message ?? {};
        if (!conversations.has(conversation)) return;
        const history = thread(conversation);
        if (message.type === 'resume' && Number.isSafeInteger(message.after) && /^[a-f0-9]{32}$/.test(message.sender)) {
          if (greet) retain(conversation, 'message', 'welcome', 'Hello from the socket');
          const start = message.after - Number(conversation);
          if (start < 0 || start > history.log.length) {
            socket.close(4000, 'Unknown replay cursor');
            return;
          }
          if (test) {
            history.after = message.after;
            resumes.dispatchEvent(new Event(conversation));
          }
          clients.set(socket, { conversation, sender: message.sender });
          for (const frame of history.log.slice(start)) {
            if (frame.id === 'withheld' && history.holdReplay > 0) history.holdReplay--;
            else socket.send(JSON.stringify(frame));
          }
        }
        const joined = clients.get(socket);
        if (message.type !== 'send' || joined?.conversation !== conversation
            || !/^send-[a-f0-9]{32}$/.test(message.id) || !validBody(message.body)) return;
        if (test) history.sends.push({ id: message.id, body: message.body });
        const previous = history.byId.has(message.id);
        const frame = retain(conversation, 'sent', message.id, message.body, joined.sender);
        if (dropAck) {
          dropAck = false;
          clients.delete(socket);
          socket.close(1012, 'Fixture dropped acknowledgement');
        } else {
          if (echoBeforeAck) {
            echoBeforeAck = false;
            socket.send(JSON.stringify({ ...frame, type: 'message' }));
          }
          socket.send(JSON.stringify({ ...frame, type: 'ack' }));
        }
        if (!previous) broadcast(conversation, frame, socket);
      },
      close(socket) { clients.delete(socket); },
    },
  });
  return {
    url: server.url.origin.replace('http:', 'ws:'),
    close() { server.stop(true); },
  };
}

if (import.meta.main) {
  const test = process.argv.includes('--test');
  const fixture = serveMessages({ port: Number(process.env.PORT ?? 48732), greet: !test, test });
  console.log(`MESSAGE_SOCKET=${fixture.url}`);
}
