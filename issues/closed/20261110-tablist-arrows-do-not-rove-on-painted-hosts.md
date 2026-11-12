# A tablist on the painted hosts has no arrow keys, makes every tab a Tab stop, and loses its tab count in a modal dialog

**Status:** Closed
**Resolution:** Painted tablists move focus by arrow keys without activating, expose one Tab stop, and report tab counts inside modal dialogs.
**Systems:** host/linux/src/presenter/typing.rs, host/linux/src/presenter/radio.rs, host/linux/src/presenter/access.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. The presenter in `host/linux/src/presenter/`
is the one Linux, Windows and Android link.

The web host gives a `role="tablist"` roving focus (LLP 1115.000 D4.5; `roveArrow`
and `tabStops`, `host/web/input-glue.js:59` to 94): Left and Right move the focus
along the tabs (Up and Down when it is vertical), Home and End go to its ends, it
wraps, nothing is pressed, and the tablist is one Tab stop, its selected tab.

The painted presenter has the radio group's half of that (`radio_key` and
`radio_tab_stop`, `presenter/radio.rs`) and nothing for a tablist: `key_default`
(`presenter/typing.rs:718`) has no branch for a focused tab's arrows, and `tabbable`
(line 478) makes every tab a stop. Shelf's segmented controls are tablists in the page
(`apps/shelf/app.contract:454`, 698, 982).

For assistive technology, `access_tree` counts a tablist's tabs with `access_sets`
only on the page's path (`presenter/access.rs:1203`). With a modal box open it walks
the box and returns first (lines 1142 to 1156), so a tablist in a dialog and its tabs
keep `set == (0, 0)`. Windows, which presents an authored dialog and reads this tree,
gives a node its position and set size only above zero
(`host/windows/src/accessibility.rs:421`).

## The failure

A button `before`, a `row role="tablist"` of three `button role="tab"` (`one`, `two`,
`three`; `one` selected), a button `after`:

| Interaction | Observed | Expected |
| --- | --- | --- |
| Focus on `one`: ArrowRight, ArrowRight, End, Home, ArrowLeft | the focus stays on `one` | `two`, `three`, `three`, `one`, `three`; nothing pressed |
| The same tablist as a `column` with `aria-orientation="vertical"`: ArrowDown, ArrowDown, End, Home, ArrowUp | the focus stays on `one` | the same five |
| From no focus: Tab four times | `before`, `one`, `two`, `three` | `before`, `one`, `after`, `before` |
| A tablist of two tabs in a `dialog` opened with `command="show-modal"`: `access_tree()` | tablist `(0, 0)`, tabs `(0, 0)`, `(0, 0)` | `(0, 2)`, `(1, 2)`, `(2, 2)`, as the same tabs on the page are told |

Enter on a focused tab presses it, as expected.

Expected: the web host's behaviour above, which is the WAI-ARIA Authoring Practices'
tabs pattern. LLP 1115.004 D9.1 decides it for Windows: "the arrow keys within a group
(a radio group, a tab list, a segmented control), as WinUI's controls take them".

## Verification

Temporary tests in `exact-linux`'s library booted each form with the CPU painter and
sent the keys through `key_down`. They printed the focus after each key as in the
table, with the press count still 0 after the arrows, and for the dialog (the
presenter told `serve_overlays()`, as the Windows host tells it)
`page tabs: [("Page views", (0, 2)), ("P one", (1, 2)), ("P two", (2, 2))]` then
`dialog tabs: [("Views", (0, 0)), ("One", (0, 0)), ("Two", (0, 0))]`. The web's
behaviour was read from `input-glue.js`, not run. Not run on a display or under
Narrator.

## Direction

Beside `radio_key` and `radio_tab_stop`, give a focused tab the same pair for its
tablist: the arrows by its orientation, Home and End, wrapping past disabled tabs,
moving the focus only; and one Tab stop, the selected tab or the one the arrows left.
Call `Self::access_sets(&mut b.out)` before the modal branch of `access_tree` returns.

## Related

LLP 1115.004 D9.1 puts the arrows in stage S1, and nothing records them as built or
owed. LLP 1115.000's ruling 5 ("the painted desktop hosts take the web's focus rule")
was built for a route's arrival (`presenter/arrive.rs`); `QUEUE.md`, "The focus
follows the route on the painted hosts, what is owed" lists four items, none a
tablist's arrows. `QUEUE.md`, "Linux and Windows: what an authored dialog or popover
still lacks" does not name the tab count.
