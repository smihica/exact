# Each reload of a cached page starts another best-compression thread, with no bound on how many run

**Status:** Closed
**Resolution:** One worker thread makes every kept page's best variants, and a page replaced before its turn is skipped and its body dropped.
**Systems:** host/render/src/serve.rs (`render_kept`), host/render/src/encode.rs (`Best::of`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The render server (`<app>-render --serve`, LLP 1048.000 D10) keeps a `render=cached` route's page, and `render_kept` makes the page's best variants (brotli at quality 11, gzip at 9) on a thread of its own, "made here once, off every request's path" (`host/render/src/serve.rs:978-984`).

A request with `Cache-Control: no-cache`, `no-store` or `max-age=0` skips the kept page and renders again (`serve.rs:703-711`, `:902-904`). A browser's reload sends `max-age=0`. So every such request goes through `render_kept`, replaces the kept page, and spawns one more `exact-render-best` thread. Nothing limits how many run, and nothing cancels the one whose page was just replaced.

Renders are bounded: "a fixed set of workers behind a bounded queue: a request that finds the queue full is a 503 at once" (`serve.rs:9-10`). The compression each render starts is not, and it takes tens of times as long as the render. One reload wastes one compression. Reloads that arrive faster than a compression finishes accumulate threads, each with its own brotli encoder, so a cached route handles reloads worse than a `render=request` route does.

The dist's files have the other shape: "compressed once by one thread from bind" (`host/render/src/encode.rs:223`).

## The failure

The server test's blog (`host/render/tests/it/serve.rs`) with a post whose page is 152 KB, in a release build. The page renders in 1.3 ms and its best variants take 62 ms.

One client sends `GET /post/big` (`render=cached`) with `Cache-Control: max-age=0`, each request after the last was answered:

```
40 reloads: answered in 252 ms; 9 compression threads at the peak
the same 40 requests without the header: 185 ms, none
10 reloads 500 ms apart: never more than 1
```

Several clients do the same, 300 requests each. `/live/big` is the same page on a `render=request` route:

| route | clients | answered in | server threads above idle at the peak | resident memory at the peak |
|---|---|---|---|---|
| `/post/big` | 2 | 0.98 s | 81 | 2,573 MB |
| `/post/big` | 8 | 3.84 s | 112 | 3,092 MB |
| `/live/big` | 8 | 0.67 s | 0 | 77 MB |

Idle, the server held 62 to 68 MB. No request was answered 503.

Expected: at most one compression in progress for a route (or one worker for all), and none for a page that has been replaced.

## Verification

Appended tests to `host/render/tests/it/serve.rs` with a data source that answers a 20 KB title, sampled the process's threads with `ps -M` (less the test's own) and its resident memory with `ps -o rss`, and ran `cargo test --release -p exact-render --test it <name> -- --nocapture --test-threads 1` on a 36-core Mac. They printed the figures above. A 1 MB page gave 202 ms a compression and 14 threads at the peak for the single client. No deployed server was measured.

## Direction

Make the best variants on one thread that takes the newest kept page of each route and drops a request for a page that is no longer kept, as the dist's files are compressed by one thread.

## Related

`issues/20261110-conditional-navigation-of-cached-route-is-flushed.md` is another defect of `render_kept` (a conditional navigation answered 200). Nothing names `exact-render-best`.
