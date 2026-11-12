# The Rust render host folds a text that has a `:hover` look, so the look never paints

**Status:** Closed
**Resolution:** Both document projections keep a text with state looks in its own box, using the shared folding rule.
**Systems:** host/web/src/document.rs, host/web/src/element.rs, kernel/src/node.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A text that is its box's only child, with no style row, no prop but its text
and no handler, is folded: `display:contents` on its element and
`display:block` on its box (LLP 1007.001). The live host and the JS template
add a condition: the text has no state look, because "a look paints its own
box" (`host/web/src/element.rs:116-129` `folded`, `:132-151` `holds_folded`;
`host/web/src/template.rs:76-79` calls both).

The document writer does not apply it. `Walk::element` calls `folds` directly
(`host/web/src/document.rs:695-702` for the box, `:979-982` for the text), and
`folds` reads a `NodeFacts`, which carries no looks
(`kernel/src/node.rs:12-28`). So the text is a box on a page the runtime
builds and no box on a page the Rust render host writes.

## The failure

```
routes nav
  home "/" render=build

component App
  view
    column testId="box"
      text "hi"
        :hover
          background-color = "#00ff00"
```

`bun host/web-js/build.mjs caltrain --contract app.contract --render rust`
writes:

```
<div data-testid="box" class="c1" style="display:block;" …><div data-exact-text="" style="display:contents;" …>hi</div></div>
```

`--render js` writes the text with no `display:contents` and the box with no
`display:block`.

A `display:contents` element generates no box (CSS Display 3 §2.5), so the
look's `background-color`, border and shadow paint nothing. Adoption keeps the
document's style, on both targets, so the look stays unpainted after the
runtime has the page.

## Verification

Ran both builds and loaded each page in the pinned Chromium (156) with the
pointer over the text. Rust-rendered: computed `display: contents`, no client
rects, and 0 green pixels of 14,400 in a screenshot of the box, as served and
after the runtime logged `adopted the document` (when the computed
`background-color` was `rgb(0, 255, 0)`). Rendered by `--render js`, and built
by the runtime from `shell.html`: `display: block` and 14,319 green pixels.

For the wasm target, a test over `exact_web::Host` printed the text's create
op with `"css":""` from `Host::boot`, and with `"css":"display:contents;"`
from `Host::boot_checkpoint` over the document's checkpoint, which reported
`adopted:true` (`host/web/src/host.rs:1302-1308` takes the walk's CSS).

## Direction

Give the walk the fact: a `has_looks` bit in `NodeFacts` (set from
`NodeRef::looks`, and by the kernel-free `DocTree`), read inside `folds` so
its callers cannot differ. `folded` and `holds_folded` then drop their own
checks.

## Related

QUEUE "A folded text's element is still styled" is about the cost of a folded
text's element. No issue names looks and folding.
