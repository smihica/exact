# On a rendered page a Markdown link that leaves the app opens in the app's own tab

**Status:** Closed
**Resolution:** The Rust document writer gives a Markdown link that leaves the app `target="_blank"` and `rel="external noopener"`, and the JS render server's DOM reflects `target` as an attribute, so renderMarkup's is written.
**Systems:** host/web/src/document.rs, host/web-js/dom.js, host/web-js/markdown.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A link to an absolute URL leaves the app, as natively: the web gives it
`target="_blank"` and `rel="external noopener"`. A link node gets them from
`props_of` (`host/web/src/element.rs:1356-1370`). A link in `markup="markdown"`
text gets them from the page's `renderMarkup` (`host/web/navigation.js:741`).

Neither writer of a rendered page gives a Markdown link them:

- the Rust document writer's restatement of `renderMarkup` writes only `href`
  (`host/web/src/document.rs:1177-1179`);
- the JavaScript render server runs `renderMarkup` over a DOM whose elements
  reflect `href` as an attribute (`host/web-js/dom.js:110`) and not `target`
  or `rel`, which `renderMarkup` sets as properties.

Adoption does not repair it: a Markdown text that arrives with its elements is
kept as it is (`host/web-js/markdown.js:29`, `:73`), until its source changes.

## The failure

```
routes nav
  home "/" render=build

component App
  view
    column
      text "see [site](https://example.com/a)" markup="markdown" testId="md"
      link href="https://example.com/a" testId="lnk"
        text "site"
```

Built with `bun host/web-js/build.mjs caltrain --contract app.contract --render
rust`, the two links to one URL are:

```
<a href="https://example.com/a">site</a>
<a data-testid="lnk" href="https://example.com/a" rel="external noopener" target="_blank" …>
```

`--render js` writes the Markdown link the same way. Expected: it also carries
`target="_blank" rel="external noopener"`, as it does on a page the runtime
builds.

A click on it replaces the app with the other site. The capture script leaves
a click on `a[href]` to the browser, so on a page that waits for the reader
(LLP 1048.007 D1) the click does not load the runtime either.

## Verification

Ran both builds (the text also held a link to an app path) and read
`index.html`. In the pinned Chromium (156), for each renderer's page the
Markdown link's `target` was null with JavaScript off, with it on before any
input, and after a press loaded the runtime and it logged `adopted the
document`. The same app built by the runtime from `shell.html` gave it
`_blank`. The link node had `_blank` every time.

## Direction

In `Walk::markup`, push `target` and `rel` for a link whose `href` leaves the
app, by one predicate shared with `props_of` (`leaves_app`), and have
`renderMarkup` set them as attributes so the render server's DOM writes them.
`renderMarkup` requires `//` after the scheme and `leaves_app` does not
(`https:post/5`); make them one rule.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names a Markdown
link in a rendered page.
