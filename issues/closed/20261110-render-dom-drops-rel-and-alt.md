# A page rendered under Bun loses the `rel` of a bound link and the `alt` of a bound symbol, and the adopted link then keeps `target="_blank"` for an app path

**Status:** Closed
**Resolution:** The Bun render DOM reflects rel and alt, so adopted bound links clear automatic targets for app paths and unnamed symbols are decorative from first paint.
**Systems:** host/web-js/dom.js, host/web-js/rt.js, host/web-js/symbols.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The render server's DOM writes attributes, and reflects a few properties onto them: `value`, `checked`, `className`, `href` (`host/web-js/dom.js:100-110`). The runtime sets two more as properties, which that DOM keeps as plain fields and never writes:

- `e.rel`, when a bound `href` leaves the app (`host/web-js/rt.js:703`). The same line sets `target` with `setAttribute`, so the page gets `target="_blank"` alone.
- `e.alt = ""`, for an image whose bound source is a symbol (`host/web-js/symbols.js:102`).

The first also misleads the runtime that adopts the page. `P` manages `target` only while the link has none or `e.rel === "external noopener"` (`rt.js:703`); a rendered link has a `target` and no `rel`, so it is read as the author's and left alone from then on.

## The failure

```
routes nav
  home "/" render=build
  about "/about" render=build

component App
  state url = "https://example.com/a"
  state role = "person"
  action local
    url = "/about"
  view
    column
      link href=url testId="lnk"
        text "site"
      image `symbol:${role}` testId="sym"
      button press=local testId="local"
        text "make it local"
```

`bun host/web-js/build.mjs <app> --render js` writes

```
<a … data-testid="lnk" … href="https://example.com/a" target="_blank" …>
<img … data-testid="sym" … data-symbol-source="symbol:person" … src="data:…" …>
```

The native render entry writes `rel="external noopener" target="_blank"` and `alt=""` for the same plan, as both renderers do when the `href` and the source are literals.

In Chromium:

| | the link | the symbol |
|---|---|---|
| rendered under Bun, as loaded | `target="_blank"`, no `rel` | no `alt`; an image in the accessibility tree |
| after a press wakes the runtime and it adopts the page | the same | `alt=""`; not in the tree |
| after `local` sets the `href` to `/about` | `href="/about" target="_blank"` | |
| drawn in the browser (`--render none`), after `local` | `href="/about"`, no `target`, no `rel` | `alt=""` throughout |

So the app's own route opens in a new tab, and until the runtime boots a screen reader is given an image the runtime marks decorative. A page that waits for the reader (LLP 1048.007 D1) keeps it until the first act.

## Verification

Built the app with `--render js`, ran `target/host-dev/caltrain-render --plan dist/app.plan --shell dist/shell.html --build` over the same plan, and diffed the two documents: the tags above. The table is the attributes read in the pinned Chromium from both builds, and the images CDP `Accessibility.getFullAXTree` reports as not ignored.

## Direction

Reflect `rel` and `alt` in `dom.js` as `href` is reflected (or have `P` and `symbols.js` call `setAttribute`).

## Related

`issues/20261110-document-markdown-external-link-has-no-target.md` is a Markdown link, whose `target` and `rel` `renderMarkup` sets as properties; its direction changes `renderMarkup`, which leaves these two writers as they are. `issues/20261110-js-render-server-links-markdown-paths-to-render-invalid.md` is the `href` the same DOM does reflect. `issues/20261110-document-select-option-without-value-not-selected.md` is another property `dom.js` reads differently.
