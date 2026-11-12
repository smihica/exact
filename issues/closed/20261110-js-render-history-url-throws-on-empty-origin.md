# The JavaScript render server's empty `location.origin` makes `historyURL` throw, so `web-build` fails

**Status:** Closed
**Resolution:** The render server's location has an origin (`ab72e13e`), and `historyURL` resolves the route against the page's URL, so a location with an `href` and no origin also works.
**Systems:** host/web/navigation.js (`historyURL`), host/web-js/render.mjs
**Severity:** P1
**Author:** Claude (Opus 5.5), from sharing the Softwear template
**Date:** 2026-11-10
**Related:** issues/20261110-js-render-server-links-markdown-paths-to-render-invalid.md

## What is wrong

Since `4f5cc4abf1`, `historyURL` builds the route's URL from the page's origin:

```js
const facts = …, url = new URL(location.href), route = new URL(location.origin + path);
```

The JavaScript render server gives the page a placeholder location whose origin
is empty (`host/web-js/render.mjs`):

```js
const location = { pathname: '/', search: '', href: 'http://render.invalid/', origin: '' };
```

So `new URL('' + '/')` throws `TypeError: Invalid URL` (`input: "/"`,
`ERR_INVALID_URL`) whenever a render commits a history entry. The JavaScript
render server renders every app with no Rust render entry, at build and in
`serve`.

## The failure

The Softwear template in `expo/exact-templates` (a store with tabs and a
router, no `app-render` entry) builds and passes its 28 web tests on
`1cbce180d4`, the parent of `4f5cc4abf1`. On `336a059fd1`, after
`4f5cc4abf1`, `bun exact.mjs web-build` and `bun exact.mjs test web` stop at
the error below; `historyURL` and the render server's location are the same
at `bb85428a87`:

```
app: no app-render entry, so its pages render under Bun (--render js)
TypeError: Invalid URL
app: the web build (the JS target) failed; the wasm target is internal (--wasm)
```

Running `host/web-js/build.mjs app --out <dir> --render js` directly gives
the stack:

```
TypeError: Invalid URL
 input: "/",
  code: "ERR_INVALID_URL"
      at historyURL (server.js:539:93)
      at commit (server.js:688:54)
      at drain (server.js:771:17)
      at apply (server.js:1275:16)
      …
      at router (server.js:7682:11)
```

To reproduce: `bun new.mjs softwear <path> --name Softwear --bundle-id
com.example.softwear --exact2 <checkout at main>` in `expo/exact-templates`,
then `bun exact.mjs web-build --out dist` in the new app.

## Fix

Either give the render server's location a real origin
(`origin: 'http://render.invalid'`, matching its `href` and `baseURI`), or
build the route's URL against `location.href` (`new URL(path,
location.href)`), which a placeholder location already supports. A test
that renders a routed page under `--render js` would have caught it.
