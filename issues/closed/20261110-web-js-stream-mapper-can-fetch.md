# On the JS target a stream's mapper can `fetch` and use storage, which the native runtime and LLP 1016.000 refuse

**Status:** Closed
**Resolution:** The JS target marks every mapper call (`answering.mapperCall`), carries the mark across the call's awaits through the bundle's async context, and refuses `fetch`, timers and storage under it with Hermes's words.
**Systems:** host/web-js/ts-stream.js, host/web-js/ts-fetch.js, host/web-js/ts-data.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1016.000 D7 (the amendment of 2026-11-10) says: "`fetch`, timers and storage are refused in a mapper." `docs/reference.md:1329-1330` says the same, with no exception for the web. The amendment of 2026-10-16 lets a socket's mapper `send` and `close` on its own socket, and the one of 2026-10-18 lets an answer in flight fetch and send in its own turns; neither gives a mapper call a fetch.

Natively every mapper call is a call marked `mapper` (`js/src/prelude.js:1371`), in which `fetch` rejects (`prelude.js:572`) and storage is refused (`prelude.js:835-836`).

The JS target marks a mapper call only while a WebSocket's `open` or `message` frame is answered, up to the call's first `await` (`host/web-js/ts-stream.js:166`, `:176`), and that mark is all `fetch` looks at (`host/web-js/ts-fetch.js:51`). In the mapper of a server-sent event stream, a byte stream, a long poll or an upload, in any mapper's call for the stream's end, and in a socket's mapper after an `await`, no answer is current, so the fetch goes out as one tied to no answer (`ts-fetch.js:39-43`). Storage has no mapper check (`host/web-js/ts-data.js:402-409`). Timers are refused, since no answer is running (`ts-fetch.js:168`).

The dev loop runs the JS target, so a mapper that fetches works while it is written and fails on a device.

## The failure

```ts
export function answer(name: string, args: any[], store: any, storage: any): any {
  return fetch(ORIGIN + '/events', { exactStream: async (event: any) => {
    if (event.type !== 'message') return 'ended';
    const r = await fetch('/assets/x.txt').then(r => r.text(), e => e.message);
    await storage.fs.writeFile('app:/data/m.txt', new TextEncoder().encode('kept'));
    return r;
  } });
}
```

read by `resource feed = feed() as shape string`, with grants for the origin and the file, and one event sent.

| in the mapper's call for a message | JS target, in Chromium | native, through the runner |
|---|---|---|
| events: `fetch` | resolves with the asset; the server is asked once | rejects, `fetch() is unavailable in a stream's mapper: fetch in the answer that opens the stream`; no request |
| events: `storage.fs.writeFile` | writes; `readFile` reads it back | rejects, `storage.writeFile() called in a stream's mapper: it was never run. Start storage inside an answer` |
| WebSocket: `fetch` before an `await` | rejects, `… in a socket's mapper: it answers each frame now` | the same words (read) |
| WebSocket: `fetch` after an `await` | resolves with the asset; the server is asked once | rejects, `… in a stream's mapper …` |

`setTimeout` throws on both.

## Verification

The JS target: one app with both streams, built with `bun host/web/build.mjs <app> --out dist` and opened in the pinned Chromium with no `?agent`; its Bun server sent one event and one socket message and counted the asset requests.

Native: `cargo test -p exact-js --test it a_later_turn_sends_and_fetch_and_timers_are_refused` passes (the last row). For the first two rows a temporary test in `js/tests/it/mapper.rs` opened an `https:` stream of the fixture and sent it `fetch 0 f` and a storage verb added to `js/tests/fixtures/mapper.ts`; it printed the two messages and left no request out. Both were removed.

## Direction

Mark every mapper call, not only a socket frame's (a flag beside `answering.mapper`, kept across the call's awaits by the async context as `answering.call` is), and refuse `fetch` and storage under it with the prelude's words.

## Related

`issues/20261110-stream-pending-ends-before-awaiting-mapper-answers.md` and `issues/20261110-upload-mapper-turn-ends-upload-mark.md` are the runner's handling of an awaiting mapper. `issues/closed/20261023-a-stream-mapper-cannot-yield.md` added it. None names what a mapper may call on the web.
