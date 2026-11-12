# A 304 for a page kept per viewport class leaves out the `Vary` its 200 carried

**Status:** Closed
**Resolution:** A page's 304 sends the viewport `Vary` its 200 sends, as RFC 9110 §15.4.5 requires.
**Systems:** host/render/src/serve.rs, host/render/src/viewport.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A page whose plan reads `exactViewport()` is rendered for the reader's viewport
and answers `Vary: Sec-CH-Viewport-Width, Sec-CH-UA-Mobile, User-Agent` (LLP
1048.006; `host/render/src/viewport.rs:65-86`). That line is not one of the
response's headers. It is in `Shared::page_headers`, which `Response::write`
appends to a page's 200 (`host/render/src/serve.rs:481-490`).

The 304 branch returns before that. It writes the `ETag`, `Cache-Control`,
`Vary`, `Expires` and `Age` found in the response's own headers
(`serve.rs:464-474`), so the only `Vary` it sends is `Accept-Encoding`.

RFC 9110 §15.4.5: "The server generating a 304 response MUST generate any of
the following header fields that would have been sent in a 200 (OK) response
to the same request", and its list begins "Content-Location, Date, ETag, and
Vary". A cache that validates updates its stored response's header fields
from the 304 (RFC 9111 §4.3.4, §3.2: "replacing field values that are already
present"). One that replaces `Vary` then holds the phone's page under `Vary:
Accept-Encoding` and can serve it to a desktop. LLP 1048.006 names a shared
cache in front of this server.

## The failure

```
routes nav
  tab kept "/kept" render=cached
  notfound render=build

shape Viewport
  width: number

component Page
  resource viewport = exactViewport() as shape Viewport
  derive narrow = viewport.width <= 700
  view
    column
      if narrow
        text "narrow"
      if not narrow
        text "wide"
```

`GET /kept` with `Sec-CH-Viewport-Width: 390`:

```
HTTP/1.1 200 OK
Cache-Control: public, max-age=0, s-maxage=4, stale-while-revalidate=4
ETag: "8bc7cd6ef37d179041aadd3a27836b80"
Vary: Accept-Encoding
Accept-CH: Sec-CH-Viewport-Width
Vary: Sec-CH-Viewport-Width, Sec-CH-UA-Mobile, User-Agent
```

The same with `If-None-Match: "8bc7cd6ef37d179041aadd3a27836b80"`:

```
HTTP/1.1 304 Not Modified
Cache-Control: public, max-age=0, s-maxage=4, stale-while-revalidate=4
ETag: "8bc7cd6ef37d179041aadd3a27836b80"
Age: 0
Vary: Accept-Encoding
Connection: keep-alive
```

Expected: the 304 also carries `Vary: Sec-CH-Viewport-Width, Sec-CH-UA-Mobile,
User-Agent`. With `Sec-CH-Viewport-Width: 1200` the same URL answers a
different page (`wide`, ETag `"bd42aae9…"`).

## Verification

A temporary test in `host/render/tests/it` served a JS-target build of a plan
holding this route with `exact_render::Server` (lifetime 4 s); `curl` printed
the headers above (the CSP and the other page headers left out). A second
test over the wasm shell, with the route at `/`, printed the same two `Vary`
lines on the 200 and one on the 304. Not run: a CDN in front of the server.

## Direction

Make the viewport `Vary` a header of the page's response where the page is
made (`document`), so the kept copy, its 304 and a flushed page all carry it,
and leave in `page_headers` the lines that belong only on a 200 (`Accept-CH`,
`Permissions-Policy`).

## Related

`a_page_is_rendered_for_its_reader_s_viewport_and_kept_per_class`
(`host/render/tests/it/viewport.rs`) checks the 200's `Vary` only. No issue or
QUEUE entry names a 304's.
