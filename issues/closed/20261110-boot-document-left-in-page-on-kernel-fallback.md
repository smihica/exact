# A `paint=boot` page keeps its boot document when the settled page needs a kernel

**Status:** Closed
**Resolution:** Kernel fallback replaces the boot document without repeating its head; cached pages contain only the settled document.
**Systems:** host/render/src/direct.rs, host/render/src/serve.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`render_js` sends a `paint=boot` route's boot document as soon as the runner
boots (`host/render/src/direct.rs:59-93`), then settles. If the settled tree is
one the kernel-free writer does not cover, it returns `Ok(None)`
(`direct.rs:105-112`). Its contract for `Ok(None)` is "nothing was sent, and
the caller renders it with a kernel" (`direct.rs:26-28`), and `document` does
that: it composes `head + body_js(...)` and sends the whole rest of the page
(`host/render/src/serve.rs:1242-1268`, `:1311-1314`).

So the response holds the boot document and then a complete second page body.
The style that hides the boot document (`SHELL_HIDDEN`) and the script that
removes it (`boot_swap_js`) are written only on the direct path
(`direct.rs:156-171`, `:197-200`).

It is reached when content the fold refuses exists only after the answers: a
`rem` or `em` length (`runner/src/instance/document.rs:481`) or a virtualized
list in rows the data fills.

## The failure

```
routes nav
  tab home "/" render=request paint=boot
  notfound render=build

component Cards
  resource cards = cards() as shape string[]
  view
    column
      head title="Cards"
      if cards.length == 0
        text "Loading cards" testId="loading"
      for c in cards key = c
        text c font-size="1.5rem"
```

with a `cards` source that answers after 300 ms, a JS-target build, and a
navigation (`Sec-Fetch-Dest: document`). The page received, abridged:

```
<title>Cards</title><meta name="viewport" …>…
<div id="exact-root" data-boot=""><div …><div … data-testid="loading" …>Loading cards</div></div></div><title>Cards</title><meta name="viewport" …>…
<style data-exact-page>…</style>
<div id="exact-root"><div …><div … class="c3" …>first</div><div … class="c3" …>second</div></div></div>
<script type="application/vnd.exact.checkpoint" …>
```

Two `#exact-root` elements, two titles, two viewport metas, no hiding style,
no swap script. Each root is the viewport's height, so:

- without JavaScript the reader sees "Loading cards", with the cards a screen
  below;
- once the runtime loads it takes the first root, logs `refused adopt:
  adoption: expected <div>, found nothing`, and builds the page there. The
  settled root stays below it, so the cards are on the page twice.

The same route with `text c` in place of the `rem` row sends one visible root
and the runtime logs `adopted the document`.

## Verification

A temporary test in `host/render/tests/it` served a JS-target build of a plan
holding this route and one without the `rem` row, with `exact_render::Server`.
`curl -H 'Sec-Fetch-Dest: document'` printed the page above. The pinned
Chromium (156), at 600 by 300, reported two roots at tops 0 and 300, a scroll
height of 600, and the texts and journal lines quoted, with JavaScript off and
on.

## Direction

Either decide before sending the boot document that the settled page can be
written without a kernel, or tell the caller a boot document went out so the
kernel path writes what follows as the direct path does: `SHELL_HIDDEN`, the
head's fields not repeated, and `boot_swap_js` after the settled root.

## Related

`host/render/tests/it/boot.rs` covers the direct path only. Nothing in
`issues/` or `QUEUE.md` names a boot document with a kernel render.
