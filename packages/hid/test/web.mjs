// @exact/hid's web conformance suite in Chromium (LLP 1133.037 §16 stage 1), over a fake `navigator.hid`
// (`packages/serial/test/chromium.mjs`): the chooser asked with the app's filters and every pad usage excluded (D11),
// a pad never offered, a lease per open on one shared open, input reports as a batch (the latest first, then every
// report since, `dropped` past `history`), output and feature reports, an unplug, and cleanup on close and forget.
//
// usage: bun packages/hid/test/web.mjs
import { checker, open } from '../../serial/test/chromium.mjs';

const fake = () => {
  class Device extends EventTarget {
    constructor(vendorId, productId, productName, collections) {
      super();
      Object.assign(this, { vendorId, productId, productName, collections, opened: false, opens: 0, sent: [], calls: [], forgotten: false });
      this.inputListeners = new Set();
    }
    addEventListener(type, listener, options) {
      if (type === 'inputreport') this.inputListeners.add(listener);
      super.addEventListener(type, listener, options);
    }
    removeEventListener(type, listener, options) {
      if (type === 'inputreport') this.inputListeners.delete(listener);
      super.removeEventListener(type, listener, options);
    }
    async open() { this.opened = true; this.opens++; }
    async close() { this.calls.push('close'); this.opened = false; }
    async forget() { this.calls.push('forget'); this.forgotten = true; this.opened = false; }
    holdNext(method) {
      const original = this[method];
      let start, finish;
      const started = new Promise(resolve => { start = resolve; });
      const held = new Promise(resolve => { finish = resolve; });
      this[method] = async (...args) => {
        this[method] = original;
        start();
        await held;
        return original.apply(this, args);
      };
      return { started, finish };
    }
    async sendReport(reportId, data) { this.calls.push('sendReport'); this.sent.push(['output', reportId, [...data]]); }
    async sendFeatureReport(reportId, data) { this.calls.push('sendFeatureReport'); this.sent.push(['feature', reportId, [...data]]); }
    async receiveFeatureReport(reportId) {
      this.calls.push('receiveFeatureReport');
      if (reportId !== 1) throw new DOMException('no such report', 'NotAllowedError');
      return new DataView(Uint8Array.of(1, 0x63, 0xff).buffer);
    }
    report(reportId, bytes) {
      const e = new Event('inputreport');
      Object.assign(e, { reportId, data: new DataView(Uint8Array.from(bytes).buffer) });
      this.dispatchEvent(e);
    }
  }
  const light = new Device(0x27b8, 0x01ed, 'blink(1) mk3', [{ usagePage: 0xff00, usage: 1, featureReports: [{ reportId: 1 }], inputReports: [], outputReports: [] }]);
  const pad = new Device(0x054c, 0x0ce6, 'DualSense', [{ usagePage: 1, usage: 5, inputReports: [{ reportId: 1 }] }]);
  const granted = new Set([pad]);
  const hid = new EventTarget();
  hid.requestDevice = async (options) => {
    globalThis.__asked = options;
    const match = [light, pad].filter((d) => options.filters.some((f) => (f.vendorId ?? d.vendorId) === d.vendorId) && !options.exclusionFilters.some((x) => d.collections.some((c) => c.usagePage === x.usagePage && c.usage === x.usage)));
    match.forEach((d) => { d.forgotten = false; granted.add(d); });
    return match;
  };
  hid.getDevices = async () => [...granted].filter((d) => !d.forgotten);
  Object.defineProperty(navigator, 'hid', { value: hid });
  Object.assign(globalThis, { __light: light, __pad: pad });
};

const { check, failed } = checker('web');
const h = await open('hid', fake);
const b64 = (bytes) => Buffer.from(bytes).toString('base64');

const chosen = await h.call('requestDevice', { filters: [{ vendorId: 0x27b8, productId: null }], exclusionFilters: null });
const asked = await h.eval(() => __asked);
check('the chooser excludes every pad usage beside the app\'s filters', asked.exclusionFilters.length === 3 && asked.exclusionFilters.every((x) => x.usagePage === 1) && JSON.stringify(asked.filters) === JSON.stringify([{ vendorId: 0x27b8 }]), JSON.stringify(asked));
check('the chosen light is its record', chosen.length === 1 && chosen[0].productName === 'blink(1) mk3' && chosen[0].collections[0].featureReports[0].reportId === 1, JSON.stringify(chosen));
check('a pad the browser remembers is never offered', (await h.call('getDevices')).map((d) => d.productName).join() === 'blink(1) mk3');
const device = chosen[0].id;
for (let round = 1; round <= 4; round++) {
  const { connection } = await h.call('open', { device });
  const empty = await h.call('inputReports', { connection });
  check(`open ${round} starts with one listener and no old reports`, (await h.eval(() => __light.inputListeners.size)) === 1 && empty.reports?.length === 0, JSON.stringify(empty));
  const announcements = await h.eval((round) => {
    __changed.length = 0;
    __light.report(2, [round]);
    return __changed.filter((topic) => topic === 'hid.inputReports').length;
  }, round);
  const batch = await h.call('inputReports', { connection });
  check(`open ${round} announces and delivers each report once`, announcements === 1 && batch.reports?.length === 1 && batch.reports[0].data === b64([round]), JSON.stringify({ announcements, batch }));
  check(`close ${round} succeeds`, (await h.call('close', { connection })) === null);
  check(`close ${round} removes the listener and closes the device`, await h.eval(() => __light.inputListeners.size === 0 && !__light.opened));
  const closed = await h.call('hidConnection', { connection });
  const drained = await h.call('inputReports', { connection });
  const latest = await h.call('inputReports', { connection, reportId: 2 });
  check(`close ${round} preserves reader cursors and the latest report`, closed.open === false && closed.connected === true && drained.reports?.length === 0 && latest.reports?.[0]?.data === b64([round]) && latest.interrupted === false, JSON.stringify({ closed, drained, latest }));
  check(`close ${round} cannot release the lease twice`, (await h.call('close', { connection })).error === 'InvalidStateError');
}
check('reports after the last close announce nothing', await h.eval(() => {
  __changed.length = 0;
  __light.report(2, [0]);
  return __changed.length === 0;
}));

const opens = await h.eval(() => __light.opens);
const a = await h.call('open', { device }), b = await h.call('open', { device });
check('two opens are two leases on one open and one listener', a.connection !== b.connection && (await h.eval(() => __light.opens)) === opens + 1 && (await h.eval(() => __light.inputListeners.size)) === 1);

for (const n of [1, 2, 3]) await h.eval((n) => __light.report(2, [n]), n);
const first = await h.call('inputReports', { connection: a.connection });
check('a reader\'s first answer is the latest report', first.reports.length === 1 && first.reports[0].data === b64([3]), JSON.stringify(first));
for (const n of [4, 5, 6, 7]) await h.eval((n) => __light.report(2, [n]), n);
const next = await h.call('inputReports', { connection: a.connection });
check('the next is every report since', next.reports.map((r) => r.data).join() === [4, 5, 6, 7].map((n) => b64([n])).join() && next.dropped === 0, JSON.stringify(next));
await h.call('inputReports', { connection: a.connection, history: 2 });
for (const n of [8, 9, 10]) await h.eval((n) => __light.report(2, [n]), n);
const bounded = await h.call('inputReports', { connection: a.connection, history: 2 });
check('past history, the rest is counted dropped', bounded.reports.length === 2 && bounded.dropped === 1, JSON.stringify(bounded));

await h.call('sendFeatureReport', { connection: a.connection, reportId: 1, data: b64([0x63, 0xff, 0, 0, 0, 30, 0]) });
await h.call('sendReport', { connection: b.connection, reportId: 0, data: b64([1]) });
check('feature and output reports reach the device', JSON.stringify(await h.eval(() => __light.sent)) === JSON.stringify([['feature', 1, [0x63, 0xff, 0, 0, 0, 30, 0]], ['output', 0, [1]]]));
check('a feature report reads back', (await h.call('receiveFeatureReport', { connection: a.connection, reportId: 1 })).data === b64([1, 0x63, 0xff]));
check('a report the device lacks fails NotAllowedError', (await h.call('receiveFeatureReport', { connection: a.connection, reportId: 9 })).error === 'NotAllowedError');

await h.call('close', { connection: a.connection });
check('the device and listener stay open while a lease does', await h.eval(() => __light.opened && __light.inputListeners.size === 1));
const closed = await h.call('hidConnection', { connection: a.connection });
const closedBatch = await h.call('inputReports', { connection: a.connection });
check('a closed lease stays connected and can read the shared reports', closed.open === false && closed.connected === true && closedBatch.reports?.map(r => r.data).join() === [8, 9, 10].map(n => b64([n])).join(), JSON.stringify({ closed, closedBatch }));
check('a closed lease cannot send a report', (await h.call('sendReport', { connection: a.connection, reportId: 0, data: b64([1]) })).error === 'InvalidStateError');
await h.eval(() => { __changed.length = 0; __light.report(2, [11]); });
const remaining = await h.call('inputReports', { connection: b.connection });
check('the remaining lease still receives each report once', remaining.reports?.length === 1 && remaining.reports[0].data === b64([11]) && (await h.eval(() => __changed.filter((topic) => topic === 'hid.inputReports').length)) === 1, JSON.stringify(remaining));
const afterClose = await h.call('inputReports', { connection: a.connection });
check('a closed lease keeps its reader cursor while another lease is open', afterClose.reports?.length === 1 && afterClose.reports[0].data === b64([11]), JSON.stringify(afterClose));
await h.eval(() => { __light.report(2, [12]); __light.opened = false; const e = new Event('disconnect'); e.device = __light; navigator.hid.dispatchEvent(e); });
const gone = await h.call('inputReports', { connection: b.connection });
check('unplugged, a reader reads interrupted and a send fails NetworkError', gone.interrupted === true && gone.reports?.[0]?.data === b64([12]) && gone.dropped === 0 && (await h.call('sendReport', { connection: b.connection, reportId: 0, data: b64([1]) })).error === 'NetworkError', JSON.stringify(gone));
const unplugged = await h.call('inputReports', { connection: b.connection, reportId: 2 });
check('unplugging removes the listener and retains the latest report', (await h.eval(() => __light.inputListeners.size)) === 0 && unplugged.reports?.length === 1 && unplugged.reports[0].data === b64([12]) && unplugged.interrupted === true, JSON.stringify(unplugged));
check('close after unplug does not release the lease twice', (await h.call('close', { connection: b.connection })).error === 'InvalidStateError');
check('reports after unplug announce nothing', await h.eval(() => {
  __changed.length = 0;
  __light.report(2, [0]);
  return __changed.length === 0;
}));
await h.call('forget', { device });
check('forget forgets the device', (await h.eval(() => __light.forgotten)) === true);
check('forget keeps the unplugged handle readable without reports', (await h.eval(() => __light.inputListeners.size)) === 0 && (await h.call('inputReports', { connection: b.connection })).reports?.length === 0);

await h.call('requestDevice', { filters: [{ vendorId: 0x27b8 }] });
const c = await h.call('open', { device }), d = await h.call('open', { device });
await h.eval(() => __light.report(2, [12]));
check('forget with two live leases succeeds', (await h.call('forget', { device })) === null);
for (const { connection } of [c, d]) {
  const state = await h.call('hidConnection', { connection });
  const batch = await h.call('inputReports', { connection });
  check('forget closes each lease without reporting an unplug', state.open === false && state.connected === true && batch.reports?.length === 1 && batch.reports[0].data === b64([12]) && batch.interrupted === false, JSON.stringify({ state, batch }));
}
check('forget removes all input listeners and stops announcements', await h.eval(() => {
  __changed.length = 0;
  __light.report(2, [13]);
  return __light.inputListeners.size === 0 && __changed.length === 0;
}));
await h.call('requestDevice', { filters: [{ vendorId: 0x27b8 }] });
const reopened = await h.call('open', { device });
const empty = await h.call('inputReports', { connection: reopened.connection });
check('opening after forget has one listener and a fresh log', (await h.eval(() => __light.inputListeners.size)) === 1 && empty.reports?.length === 0, JSON.stringify(empty));
await h.call('close', { connection: reopened.connection });

const overlapping = await h.eval(async device => {
  const held = __light.holdNext('open');
  const opens = __light.opens;
  const a = __impl.open({ device });
  await held.started;
  const b = __impl.open({ device });
  held.finish();
  const leases = await Promise.all([a, b]);
  const result = { opens: __light.opens - opens, listeners: __light.inputListeners.size, leases };
  await Promise.all(leases.map(({ connection }) => __impl.close({ connection })));
  result.listenersAfterClose = __light.inputListeners.size;
  return result;
}, device);
check('overlapping opens share one device open and release one listener', overlapping.opens === 1 && overlapping.listeners === 1 && overlapping.listenersAfterClose === 0 && overlapping.leases.every(x => x.connection), JSON.stringify(overlapping));

const raced = await h.eval(async device => {
  const first = await __impl.open({ device });
  const held = __light.holdNext('close');
  const closing = __impl.close({ connection: first.connection });
  await held.started;
  const opening = __impl.open({ device });
  held.finish();
  await closing;
  const second = await opening;
  __changed.length = 0;
  __light.report(2, [14]);
  const result = {
    opened: __light.opened, listeners: __light.inputListeners.size,
    announcements: __changed.filter(topic => topic === 'hid.inputReports').length,
    batch: __impl.inputReports({ connection: second.connection }),
  };
  await __impl.close({ connection: second.connection });
  return result;
}, device);
check('reopening during close waits for the device and installs one listener', raced.opened && raced.listeners === 1 && raced.announcements === 1 && raced.batch.reports?.length === 1 && raced.batch.reports[0].data === b64([14]), JSON.stringify(raced));


for (const ending of ['close', 'forget', 'disconnect']) {
  const final = await h.eval(async ({ device, ending }) => {
    const a = await __impl.open({ device }), b = await __impl.open({ device });
    const readers = [
      { connection: a.connection, history: 2 },
      { connection: a.connection, reportId: 2 },
      { connection: b.connection, reportId: 3 },
    ];
    readers.forEach(args => __impl.inputReports(args));
    for (const [id, value] of [[2, 21], [3, 22], [2, 23]]) __light.report(id, [value]);
    if (ending === 'close') {
      await __impl.close({ connection: a.connection });
      await __impl.close({ connection: b.connection });
    } else if (ending === 'forget') await __impl.forget({ device });
    else {
      __light.opened = false;
      const e = new Event('disconnect'); e.device = __light; navigator.hid.dispatchEvent(e);
    }
    const batches = readers.map(args => __impl.inputReports(args));
    const drained = readers.map(args => __impl.inputReports(args));
    const latest = __impl.inputReports({ connection: b.connection });
    return { batches, drained, latest, listeners: __light.inputListeners.size };
  }, { device, ending });
  const values = final.batches.map(batch => batch.reports?.map(r => r.data));
  check(`${ending} preserves every reader's final batch and dropped count`,
    JSON.stringify(values) === JSON.stringify([[22, 23], [21, 23], [22]].map(values => values.map(n => b64([n])))) &&
    final.batches.map(b => b.dropped).join() === '1,0,0' && final.batches.every(b => b.interrupted === (ending === 'disconnect')) &&
    final.drained.every(b => b.reports?.length === 0 && b.dropped === 0) && final.latest.reports?.[0]?.data === b64([23]) && final.listeners === 0,
    JSON.stringify(final));
  await h.call('requestDevice', { filters: [{ vendorId: 0x27b8 }] });
}

for (const ending of ['close', 'forget']) {
  for (const method of ['sendReport', 'sendFeatureReport', 'receiveFeatureReport']) {
    const after = await h.eval(async ({ device, ending, method }) => {
      const { connection } = await __impl.open({ device });
      __light.calls.length = 0;
      const closing = __impl[ending](ending === 'close' ? { connection } : { device });
      const report = __impl[method]({ connection, reportId: 1, data: 'AQ==' });
      return { ended: await closing, report: await report, calls: __light.calls };
    }, { device, ending, method });
    check(`${method} invoked after ${ending} is rejected before reaching the device`,
      after.ended === null && after.report?.error === 'InvalidStateError' && after.calls.join() === ending, JSON.stringify(after));
    await h.call('requestDevice', { filters: [{ vendorId: 0x27b8 }] });

    const before = await h.eval(async ({ device, ending, method }) => {
      const { connection } = await __impl.open({ device });
      __light.calls.length = 0;
      const held = __light.holdNext(method);
      const report = __impl[method]({ connection, reportId: 1, data: 'AQ==' });
      await held.started;
      const closing = __impl[ending](ending === 'close' ? { connection } : { device });
      held.finish();
      return { report: await report, ended: await closing, calls: __light.calls };
    }, { device, ending, method });
    check(`${ending} waits for an earlier ${method} to finish`, before.ended === null &&
      (method === 'receiveFeatureReport' ? before.report?.data === b64([1, 0x63, 0xff]) : before.report === null) &&
      before.calls.join() === `${method},${ending}`, JSON.stringify(before));
    await h.call('requestDevice', { filters: [{ vendorId: 0x27b8 }] });
  }
}

const rejectedForget = await h.eval(async device => {
  const { connection } = await __impl.open({ device });
  __impl.inputReports({ connection });
  __light.report(2, [24]);
  const forget = __light.forget;
  __light.forget = async () => { throw new DOMException('still granted', 'NotAllowedError'); };
  const result = await __impl.forget({ device });
  __light.forget = forget;
  const state = __impl.hidConnection({ connection });
  const batch = __impl.inputReports({ connection });
  const sent = await __impl.sendReport({ connection, reportId: 1, data: 'AQ==' });
  const listeners = __light.inputListeners.size;
  await __impl.close({ connection });
  return { result, state, batch, sent, listeners };
}, device);
check('a rejected forget preserves the open lease, listener and unread reports', rejectedForget.result?.error === 'NotAllowedError' &&
  rejectedForget.state.open && rejectedForget.state.connected && rejectedForget.batch.reports?.[0]?.data === b64([24]) &&
  !rejectedForget.batch.interrupted && rejectedForget.sent === null && rejectedForget.listeners === 1, JSON.stringify(rejectedForget));

const disposed = await h.eval(async device => {
  const a = await __impl.open({ device }), b = await __impl.open({ device });
  await __impl.dispose?.([a.connection]);
  const shared = { state: __impl.hidConnection({ connection: a.connection }), opened: __light.opened, listeners: __light.inputListeners.size };
  __light.report(2, [25]);
  const batch = __impl.inputReports({ connection: b.connection });
  await __impl.dispose?.([a.connection, b.connection, 'missing']);
  const ended = { opened: __light.opened, listeners: __light.inputListeners.size, batch: __impl.inputReports({ connection: b.connection }) };
  // Also clean up when testing an implementation without disposal.
  await __impl.close({ connection: a.connection });
  await __impl.close({ connection: b.connection });
  return { shared, batch, ended };
}, device);
check('disposing an undelivered handle releases only its lease; the last removes the listener',
  !disposed.shared.state.open && !disposed.shared.state.connected && disposed.shared.opened && disposed.shared.listeners === 1 &&
  disposed.batch.reports?.[0]?.data === b64([25]) && !disposed.ended.opened && disposed.ended.listeners === 0 &&
  disposed.ended.batch.error === 'InvalidStateError', JSON.stringify(disposed));

const large = await h.eval(async device => {
  const { connection } = await __impl.open({ device });
  const bytes = Uint8Array.from({ length: 256 * 1024 }, (_, i) => i % 256);
  __light.report(2, bytes);
  const batch = __impl.inputReports({ connection });
  const data = batch.reports?.[0]?.data;
  const decoded = data ? atob(data) : '';
  await __impl.close({ connection });
  return { length: decoded.length, equal: decoded.length === bytes.length && bytes.every((byte, i) => decoded.charCodeAt(i) === byte) };
}, device);
check('large input reports are encoded without exceeding the argument limit', large.equal, JSON.stringify(large));

await h.close();
console.log(`${failed()} failed`);
process.exit(failed() ? 1 : 0);
