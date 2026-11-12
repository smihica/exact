# On Hermes, a `ReadableStream` given to `new Response` or to `fetch` as a body becomes the text `[object ReadableStream]`

**Status:** Closed
**Resolution:** Response consumes ReadableStream byte chunks, and fetch rejects stream request bodies before sending.
**Systems:** js/src/binary.js (`Response`), js/src/prelude.js (`request`), js/src/streams.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A Hermes data module has `ReadableStream`, `CompressionStream` and `DecompressionStream` (`js/src/streams.js`; LLP 1027 D10, 2026-11-10). At `dd6f5dcd` the prelude's `Response` and `fetch` convert any body that is not a string, a `URLSearchParams` or a BufferSource with `String()`: `js/src/binary.js:714-715` for `new Response(body)`, `js/src/prelude.js:602` for a request. A stream is therefore read as its `toString()`.

Fetch takes a `ReadableStream` as a body (https://fetch.spec.whatwg.org/#bodyinit-unions), and `new Response(stream).arrayBuffer()` is the usual way to collect a compression stream's output. `docs/reference.md` lists a string, `URLSearchParams` or bytes as what `new Response` takes, and LLP 1027 D10 says what is missing is missing at its first call, by name. Here nothing is thrown or journaled, so the app stores or uploads the wrong bytes, where the web build of the same `app.ts` gets the stream's.

## The failure

```ts
const bytes = new TextEncoder().encode("hello");
const stream = new ReadableStream({ start(c) { c.enqueue(bytes); c.close(); } });
const viaResponse = await new Response(stream).text();
const cs = new CompressionStream("gzip");
const w = cs.writable.getWriter(); w.write(bytes); w.close();
const packed = await new Response(cs.readable).arrayBuffer();
```

Observed: `viaResponse` is `[object ReadableStream]`; `packed.byteLength` is 23 and its bytes decode to `[object ReadableStream]`. Chrome 156 gives `hello` and a 25-byte gzip member (`1f 8b …`).

`fetch(url, { method: "POST", body: stream, duplex: "half" })` hands the runner

```
POST https://fixture.exact.test/up headers=[] body="[object ReadableStream]"
```

where a browser streams the body, and rejects with a `TypeError` when `duplex` is not given.

## Verification

Ran both snippets as data sources in a temporary `exact-js` test, through `Module::answer`: the first answered `{"viaResponse":"[object ReadableStream]","packedBytes":23,"packedText":"[object ReadableStream]"}`, and the second returned the request above. The Chrome results are from the pinned Chromium (156.0.8078.4) running the same lines.

## Direction

Have `Response` hold a `ReadableStream` body and read it to its end in `text()`, `json()` and `arrayBuffer()` (a chunk that is not bytes is a `TypeError`, as Fetch says). Refuse one as a request body by name, since the host sends a whole body. Say which of these a stream body does in the `Response` row of `docs/reference.md`.

## Related

`issues/closed/20261023-a-stream-mapper-cannot-yield.md` added the streams and does not cover bodies. No issue or QUEUE entry names it.
