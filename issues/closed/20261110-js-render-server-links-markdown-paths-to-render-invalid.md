# The JavaScript render server writes a Markdown link to an app path as `http://render.invalid/…`

**Status:** Closed
**Resolution:** `renderMarkup` writes a Markdown link's destination as authored once it is known to navigate, so a page the JS render server writes links app paths, not the placeholder base.
**Systems:** host/web-js/render.mjs, host/web/navigation.js (`renderMarkup`), host/web-js/markdown.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`renderMarkup` gives a Markdown link the absolute form of its destination:
`span.href = navigableURL(href)`, and `navigableURL` returns `new URL(href,
document.baseURI).href` (`host/web/navigation.js:704-706`, `:731`, `:740`). In
a browser the base is the page's. In the JavaScript render server it is a
placeholder, `baseURI: 'http://render.invalid/'`
(`host/web-js/render.mjs:129`), set so the runtime can check that a link
navigates. The server's DOM writes the property as the `href` attribute
(`host/web-js/dom.js:110`), so the page it sends links every app path to
`render.invalid`.

Adoption does not repair it: a Markdown text that arrives with its elements is
kept as it is (`host/web-js/markdown.js:29`, `:73`).

The JavaScript render server is what renders a site with no Rust render
entry, at build and in `serve` (LLP 1153.001).

## The failure

```
routes nav
  home "/" render=build
  other "/other" render=build

component App
  view
    column
      text "see [the other page](/other)" markup="markdown" testId="md"
```

`bun host/web-js/build.mjs caltrain --contract app.contract --render js`
writes, in `index.html`:

```
<a href="http://render.invalid/other">the other page</a>
```

`--render rust` writes `<a href="/other">the other page</a>` for the same
plan.

A click on the link asks for `http://render.invalid/other`, which fails with
`net::ERR_NAME_NOT_RESOLVED` and leaves the reader on the browser's error
page. It does so before the runtime loads and after it has adopted the page.

## Verification

Ran both builds (the text also held an external link) and read `index.html`.
In the pinned Chromium (156), on the `--render js` page of a plan with the
same link to `/wait`, the link's `href` attribute was
`http://render.invalid/wait` as served and after the runtime logged `adopted
the document`; clicking it at either time gave a failed navigation to that
URL and `chrome-error://chromewebdata/`. On the `--render rust` page the same
click went to `/wait`.

Not run: `serve`'s per-request render, which makes its document in the same
realm (`render.mjs:125-130`).

## Direction

Once `navigableURL` accepts a destination, write it as the author wrote it,
as the Rust document writer does (`host/web/src/document.rs:1177-1179`),
instead of the URL resolved against the base. In a browser the two are the
same link.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names
`render.invalid`. No app in `apps/` has a Markdown link to an app path, which
is why none shows it.
