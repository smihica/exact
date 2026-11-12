# On the web, a serial read larger than about 125,000 bytes is dropped and reported as `RangeError`

**Status:** Closed
**Resolution:** The web read loop appends a chunk byte by byte instead of spreading it as arguments, so a read as large as `bufferSize` allows arrives whole.
**Systems:** packages/serial/web/index.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`, the web implementation's read loop appends each chunk Web Serial delivers to an
array of numbers with a spread (`packages/serial/web/index.js:45-47`):

```js
const { value, done } = await reader.read();
if (done) break;
if (value?.length) { c.pending.push(...value); changed('serial.serialInput'); }
```

A spread passes one argument per byte, and Chromium refuses a call with too many:
`RangeError: Maximum call stack size exceeded`. The loop's `catch` (`:49-54`) takes that for a
read error of the port: it records `{at, code: "RangeError"}`, releases the reader and reads
on. The chunk is gone. LLP 1133.037 D8 says reading loses nothing.

A read returns what the browser has buffered, up to the port's `bufferSize`: the stream's
high-water mark is `bufferSize` and its pull step reads "up to desiredSize bytes"
([Web Serial, `readable`](https://serial.spec.whatwg.org/#dom-serialport-readable)). With the
default 255 no read is that large. The interface allows `bufferSize` up to 16 MiB
(`packages/serial/exact.json`, `open`), and with one above about 125,000 the buffer fills
whenever the page reads slower than the device sends: while the loop waits at its retained
bound (`:43`), or while the main thread is busy. An app that asks for a large buffer to keep up
with a fast device then loses a whole buffer at the first pause.

**The failure.** `open(port=…, baudRate=115200, bufferSize=16777216)`; 125,001 bytes arrive as
one read of 125,000 and one of 1.

Observed: `serialInput` answers 1 byte, `errors: [{"at":0,"code":"RangeError"}]`.
Expected: 125,001 bytes, no error.

**Verification.** In a page of the pinned Chromium (156), the implementation imported as
written, over a stand-in `navigator.serial` whose port's `readable` delivers the chunks (a real
port needs a device):

```js
const chunks = [new Uint8Array(125000).fill(0x41), Uint8Array.of(0x42)];
const port = { connected: true, getInfo: () => ({}), async close() {}, async open() {
  this.readable = new ReadableStream({ pull(c) { const v = chunks.shift(); if (v) c.enqueue(v); else return new Promise(() => {}); } }, { highWaterMark: 0 }); } };
Object.defineProperty(navigator, 'serial', { configurable: true, value: { addEventListener() {}, requestPort: async () => port } });
const impl = implementation({ changed() {} });
const opened = await impl.open({ port: (await impl.requestPort({})).id, baudRate: 115200, bufferSize: 16777216 });
// 200 ms later: impl.serialInput({ connection: opened.connection })
```

```
device sent 124001 bytes; serialInput answers 124001 bytes, errors []
device sent 125001 bytes; serialInput answers 1 bytes, errors [{"at":0,"code":"RangeError"}]
device sent 1000001 bytes; serialInput answers 1 bytes, errors [{"at":0,"code":"RangeError"}]
```

Not run: Chrome with a device.

**Direction.** Keep the undelivered bytes as a list of `Uint8Array` chunks (or one growing
buffer) and concatenate at `serialInput`; that also stops holding a boxed number per byte for up
to 16 MiB.

**Related.** `issues/20261110-level-meter-long-gap-throws.md` is the same spread in
`packages/recording/web/index.js`. `packages/usb/web/index.js` and this file's `b64` already
chunk their spreads.
