# `searchParam` on the JS target cuts the query at its second `?`

**Status:** Closed
**Resolution:** The JS searchParam reader preserves literal question marks and subsequent query parameters, matching the shared route corpus.
**Systems:** host/web-js/router.js, route/src/location.rs, route/tests/corpus.json
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `x_searchParam` (`host/web-js/router.js:122-126`) takes the query as `e[2].split("?")[1]`: the text
between the first `?` and the second. The runner's `search_param` (`route/src/location.rs:225-242`) takes everything
after the first `?` (`split_once('?')`). The runner is right: in the URL Standard a query runs from the first `?` to
the `#`, and a later `?` is data ([URL, query state](https://url.spec.whatwg.org/#query-state)). `canonical` keeps a
`?` inside a query on both targets, so an entry can hold one. The JS target then cuts the value there and loses every
parameter after it.

## The failure

```
routes nav
  tab home "/"
    login "/login"

component Probe
  derive current = top(nav)
  action go(url: string)
    nav = push(nav, url)
  view
    main navigationKey=`${current.id}` testId="navigation"
      text `url=${current.url} next=${searchParam(current, "next")}` testId="where"
      button press=go("/login?next=/home?tab=1") testId="go"
        text "go"
```

After the press the JS target shows `url=/login?next=/home?tab=1 next=/home`. Expected: `next=/home?tab=1`.

| entry URL | name | JS target | runner, and `new URL(…).searchParams.get(name)` |
|---|---|---|---|
| `/login?next=/home?tab=1` | `next` | `/home` | `/home?tab=1` |
| `/search?q=why?&page=2` | `q` | `why` | `why?` |
| `/search?q=why?&page=2` | `page` | (empty) | `2` |

## Verification

- The app above, built with `bun host/web-js/build.mjs` and driven in Chrome (`tap "go"`, then `expect text "where"`):
  `text of "where" is "url=/login?next=/home?tab=1 next=/home"`.
- `x_searchParam` called on the three rows, over the real `router.js` in Bun: the JS column.
- `search_param(&entry, name)` on the same three URLs after `canonical`, in a test of `exact-route`: the last column,
  and `canonical` returned each URL unchanged.
- `new URL("https://x" + url).searchParams.get(name)` in Bun 1.4.2: the last column.

A page opened at such an address was not run.

## Direction

Read the query as the runner does: `const at = e[2].indexOf("?"); if (at < 0) return ""; const q = e[2].slice(at + 1);`.
Add rows such as `{"url": "/?next=/home?tab=1&b=2", "name": "next", "expected": "/home?tab=1"}` and the same URL with
`"name": "b", "expected": "2"` to the `search` list of `route/tests/corpus.json`, whose twelve rows hold no second `?`.
Only `route/tests/corpus.rs` runs that list. `host/web/deep-link.test.mjs` already runs the corpus's `canonical`,
`location_of` and `matches` rows against the JS matcher, and can run `search` against `x_searchParam` the same way.

## Related

No entry in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names `searchParam`.
