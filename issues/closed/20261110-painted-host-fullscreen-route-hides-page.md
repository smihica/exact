# A full-screen route with an authored background hides the page under it on Linux, Windows and Android, where the guide says the page shows through

**Status:** Closed
**Resolution:** Full-screen routes with authored backgrounds keep the covered page and split panes visible and inert on Linux, Windows and Android
**Systems:** host/linux/src/navigation.rs (the painted presenter: Linux, Windows, Android), docs/contract-for-agents.md
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The ruling of 2026-11-10 (`issues/closed/20261027-fullscreen-route-translucent-background.md`,
commit `c3475108`): "A full-screen route that paints its own background leaves the host
surfaces behind it clear, so a translucent background, or one animating toward
transparent, composites over the page it covers, as CSS has it." Its resolution reads
"Option A, on every host that presents a full-screen route", where painting its own
background means the author wrote `background-color` (`transparent` included) or
`background-image`. The commit changed iOS, macOS and the web. The guide states the rule
with no host named (`docs/contract-for-agents.md:2711-2716`): "the page it covers stays
under it, inert, and shows through where its background is translucent or while it
fades out".

The painted presenter was not changed. `Navigation::sync`
(`host/linux/src/navigation.rs:255-263`) lets a route show what it sits over only when
its `navigationPresentation` is `"modal"`:

```rust
let sheets: Vec<bool> = routes.iter().map(|r| {
    kernel.node(*r).and_then(|n| n.props.str(PropId::NavigationPresentation)) == Some("modal")
}).collect();
```

so `route_visibility` hides the page under every `fullscreen` route, and a translucent
one is painted over the root's ground. The file says it holds "the browser's route
visibility rule" (LLP 1038 D6 has Linux adopt the web's), and the browser's is now
`overPage` (`host/web/navigation.js:29`).

## The failure

```
routes nav
  home "/"
    veil "/veil"
    full "/full"
component App
  action veil
    nav = push(nav, "/veil")
  action full
    nav = push(nav, "/full")
  action close
    nav = back(nav)
  view
    main navigationKey=`${top(nav).id}` width="100%" height="100%"
      for e in stack(nav).filter(x => x.name == "home") key=e.id
        column navigationKey=`${e.id}` position="absolute" inset=0 background-color="#ffffff"
          button "Veil" testId="veil" press=veil
          button "Full" testId="full" press=full
          box testId="page" width=200 height=200 background-color="#ff0000"
      for e in stack(nav).filter(x => x.name == "veil") key=e.id
        column navigationKey=`${e.id}` navigationPresentation="fullscreen" background-color="rgba(0, 0, 0, 0.5)" position="absolute" inset=0
          button "Close" testId="close-veil" press=close
      for e in stack(nav).filter(x => x.name == "full") key=e.id
        column navigationKey=`${e.id}` navigationPresentation="fullscreen" position="absolute" inset=0
          button "Close" testId="close-full" press=close
```

With `veil` open, `Host::route_visibility` of the red box is `(true, true)`, hidden and
inert, and the box is not painted. Expected `(false, true)`: shown and inert, the red
box under the half-black route, as on iOS, macOS and the web.

## Verification

A temporary test in `host/linux/src/presenter/scrollers/hidden_tests.rs` booted this
source at 400 by 600 with the CPU painter and pressed each route open
(`cargo test -p exact-linux --lib`). It printed:

```
full: page (hidden, inert) = (true, true)
veil: page (hidden, inert) = (true, true)
veil: the page's red box is painted = false
```

The first line is right. Not run on Windows or Android, which link this file. The
web's and the Mac's rule were read, not run.

## Direction

In `Navigation::sync`, count a `fullscreen` route as a sheet for `route_visibility`
when its style wrote `background-color` or `background-image`
(`style.written.has(StyleId::BackgroundColor)`, as `paint/neutral.rs:302` asks).
`presenter/split.rs:396-400` hides a split's list under any `fullscreen` route and
needs the same test. If the painted hosts are to stay as they are, name in the guide's
sentence the hosts it holds on.

## Related

The closed issue's header and its list of changes name iOS, macOS and the web only.
QUEUE's "A bound empty background is a clear on Apple and nothing on the web JS target"
is about a bound `""` on those hosts.
