# A page streamed to a browser has no hero image preload; the same page buffered has one

**Status:** Closed
**Resolution:** Streamed and buffered JavaScript pages share hero selection, preload the chosen image before the root, and mark it at high priority.
**Systems:** host/render/src/direct.rs, host/render/src/page.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`body_js` gives a page's first shown image `fetchpriority="high"` and writes a
`<link rel="preload" as="image">` ahead of the document
(`host/render/src/page.rs:404`, `:414-417`; `hero_image` at `:437-499`). The
kernel-free writer that streams a page to a navigation composes the same parts
itself (`body_open_js`, the root, `body_close_js`) and never calls `hero_image`
(`host/render/src/direct.rs:151-203`). Its module doc says its bytes "are the
page `body_js` composes from a kernel render, but for the digest"
(`direct.rs:9-12`).

So the hint reaches a crawler, `curl` and a CDN, and not a browser's
navigation, the request it exists for. A `cached` route keeps whichever form
its first request made (`host/render/src/serve.rs:993-1004`), so later readers
get one or the other by chance.

## The failure

```
routes nav
  tab home "/" render=request
  notfound render=build

component Hero
  view
    column
      head title="Hero"
      image "assets/caltrain.png" alt="Hero"
      text "Welcome"
```

served by the render host over a JS-target build.

`GET /` holds `<link rel="preload" as="image" href="assets/caltrain.png">` and
`<img fetchpriority="high" alt="Hero" src="assets/caltrain.png">`.

`GET /` with `Sec-Fetch-Dest: document` (chunked) holds no preload link, and
`<img alt="Hero" src="assets/caltrain.png">`.

Expected: the same page both ways.

## Verification

Built the plan with `bun host/web-js/build.mjs caltrain --contract app.contract
--render none`. A temporary test in `host/render/tests/it` served a build
holding this page with `exact_render::Server`; `curl` fetched it both ways and
printed the two results above (10,401 bytes buffered, with a `Content-Length`
and an `ETag`; chunked when streamed).

The render entry's own differential check reports it:
`target/host-dev/caltrain-render --plan dist/app.plan --shell dist/shell.html
--compare /` printed

```
{"location":"/","same":false,"differs":"streamed page at byte 300: kernel «…<link rel=\"preload\" as=\"image\" href=\"assets/caltrain.png\"><di» direct «…<div id=\"exact-root\">…»"}
```

## Direction

Find the hero from the tree before the root is written, as `before_root` finds
the keyframes and the scroller, so `direct.rs` can put the link in what it
sends ahead of the root and the walk can mark that `img`. `body_js` can then
take the same answer instead of scanning the HTML.

## Related

`host/web-js/document-render.test.mjs` checks the hero image for pages at
build, which are buffered. `issues/` and `QUEUE.md` have nothing on it.
