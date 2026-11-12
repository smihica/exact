# On the painted hosts the tab bar under a modal route still takes presses

**Status:** Closed
**Resolution:** Modal and fullscreen routes make the painted tablist and shell navigation bar inert until dismissal.
**Systems:** host/linux/src/navigation.rs (the painted presenter: Linux, Windows), host/linux/src/presenter/bars.rs (Android)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A modal route is modal. LLP 1115.000's as-built row for D4.5 says "The tab bar under a
sheet is inert", and the web does it in `host/web/navigation.js:441`
(`setInert(list, sheet(routes[selected]) || …)`, a sheet being a `modal` or
`fullscreen` route), held by `host/web/tests/route-focus.test.mjs:232`. The painted
presenter holds "the browser's route visibility rule" (`host/linux/src/navigation.rs:1`,
LLP 1038 D6) and makes the route beneath a sheet inert, but not the root's tablist:
`Navigation::sync` (`navigation.rs:179-275`) writes a `(hidden, inert)` pair for each
tabpanel (`:235-237`) and each route (`:264-268`), and nothing for `tabs.tablist`. So
`visibility_to` finds the tab bar live while a sheet is selected, and a press on
another tab switches tabs with the sheet still open on the first tab's stack.

On Windows a modal surface blocks the window: dialogs "block interactions with the app
window until being explicitly dismissed"
(https://learn.microsoft.com/windows/apps/design/controls/dialogs-and-flyouts/dialogs).

## The failure

```
routes nav
  tab home "/"
    sheet "/sheet"
  tab other "/other"
component App
  action pick(name: string)
    nav = select(nav, name)
  action open(url: string)
    nav = push(nav, url)
  action back
    nav = back(nav)
  view
    main navigationKey=`${top(nav).id}` navigationBack="back" testId="nav" width="100%" height="100%" display="flex" flex-direction="column"
      column flex=1 min-height=0 position="relative"
        for t in nav.tabs key = t.name
          column role="tabpanel" id=`panel-${t.name}` position="absolute" inset=0
            for e in t.stack key = e.id
              if e.name == "home"
                column navigationKey=`${e.id}` testId="route-home" position="absolute" inset=0
                  button press=open("/sheet") testId="open" height=40
                    text "Open"
              if e.name == "sheet"
                column navigationKey=`${e.id}` navigationPresentation="modal" testId="route-sheet" position="absolute" inset=0
                  button press=back testId="close" height=40
                    text "Close"
              if e.name == "other"
                column navigationKey=`${e.id}` testId="route-other" position="absolute" inset=0
                  text "Other" testId="other-title"
      row role="tablist" testId="tabs"
        button role="tab" aria-controls="panel-home" aria-selected=(nav.tab == "home") press=pick("home") testId="tab-home" flex=1 height=40
          text "Home"
        button role="tab" aria-controls="panel-other" aria-selected=(nav.tab == "other") press=pick("other") testId="tab-other" flex=1 height=40
          text "Other"
```

Click `open`, then click `tab-other`. Observed: with the sheet open,
`route_visibility(route-home)` is `(false, true)` and `route_visibility(tabs)` is
`(false, false)`; the hit at the middle of `tab-other` is `tab-other`; after the click
its `aria-selected` is `true` and the other tab's route is shown. Expected: the tablist
inert, the hit and the click refused, as the web's test asserts.

## Verification

A temporary test in `host/linux/src/presenter/arrive_tests.rs` booted this source at
400 by 800 with the CPU painter (`cargo test -p exact-linux --lib`) and printed:

```
sheet open: home (hidden, inert) = (false, true), tablist = (false, false), tab-other = (false, false)
tab-other rect = (200.0, 760.0, 200.0, 40.0); hit at its middle = Some(8) (tab-other is 8, sheet is 13)
before: tab-other aria-selected = Some(Bool(false))
after click: tab-other aria-selected = Some(Bool(true))
after click: other panel's route (hidden, inert) = (false, false)
```

`bun test host/web/tests/route-focus.test.mjs -t "a modal route is a dialog"` passes,
asserting `document.getElementById('tabs').inert` is `true`. Not run on Windows, which
links this file, or on Android. There the tablist is the shell's navigation bar, and by
reading `presenter/bars.rs:498-537` (`bar_choose`) presses a tab without asking whether
a sheet is open.

## Direction

In `Navigation::sync`, when the selected route of the selected stack is `modal` or
`fullscreen`, record the root's tablist as inert and not hidden, and drop the entry
when it is not, as the panels' entries are dropped. `bar_choose` needs the same
refusal for a shell's bar.

## Related

QUEUE's "The focus follows the route on the painted hosts, what is owed" lists four
gaps and not this one. "The neutral look on Linux and Windows, what is owed" says
sheets are not yet drawn there; this is the modality, which the host already applies
to the covered route.
