# A conditional navigation to a `cached` route the origin does not hold is flushed as a 200 with no validator

**Status:** Closed
**Resolution:** Conditional navigations to cached routes wait for the render on a cache miss and retain validators and the route's cache policy.
**Systems:** host/render/src/serve.rs, host/render/src/stream.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A page is sent in two parts only to a browser's navigation that is not
conditional: "never a CDN's request, a conditional one, or a `HEAD`: those wait
for the render and get its status, validators and caching as before"
(`host/render/src/stream.rs:6-9`). `document` tests that with
`request.if_none_match.is_none()` (`host/render/src/serve.rs:1189-1194`).

On a miss of a `cached` route, `render_kept` renders with a copy of the request
whose `if_none_match` is `None`, so that the render returns a body to keep, and
copies `navigate` unchanged (`serve.rs:947-960`). The copy passes the test, and
the conditional request is flushed.

## The failure

A JS-target build served by the render host, a route `kept "/kept"
render=cached`, a lifetime of 4 s. Each request carries `Sec-Fetch-Dest:
document`:

| Request | Answer |
|---|---|
| 1. `GET /kept`, nothing kept | 200, chunked, `Cache-Control: private, no-cache`, no `ETag` |
| 2. `GET /kept`, kept | 200, `Content-Length`, `Cache-Control: public, max-age=0, s-maxage=4, …`, `ETag: "dcb88a5d…"` |
| 3. the same with `If-None-Match: "dcb88a5d…"`, kept | 304 |
| 4. the same after 4.5 s, nothing kept | 200, chunked, `private, no-cache`, no `ETag` |

Without `Sec-Fetch-Dest`, request 4 gets the buffered answer: 200 with
`Content-Length`, the route's `Cache-Control` and a new `ETag`.

Expected: request 4 gets that buffered answer, as the module doc says and as
a conditional navigation to a `request` route does.

Nothing breaks, and the body is sent either way: a fresh render never matches
the validator, because the checkpoint carries the render's time (two renders
of one page 300 ms apart had different ETags). What the reader loses is the
validator. The browser stores a `private, no-cache` copy it cannot revalidate,
so its next visit asks unconditionally and downloads the kept page in full
where request 3 got a 304.

## Verification

A temporary test in `host/render/tests/it` served the build with
`exact_render::Server`; a script sent the four requests over a socket, waiting
4.5 s where the table says, and printed the heads summarised above. The log
line for request 4 ended `flushed streamed`.

## Direction

In `render_kept`, set the copy's `navigate` to `request.navigate &&
request.if_none_match.is_none()`.

## Related

`a_navigation_gets_the_head_before_the_render_and_the_rest_after`
(`host/render/tests/it/serve.rs`) covers an unconditional navigation of a
cached route. No issue or QUEUE entry names the conditional case.
