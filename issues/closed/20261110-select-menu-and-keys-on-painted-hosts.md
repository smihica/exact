# A select on the painted hosts takes no key, and its menu cannot scroll, close on Escape or Back, or reach assistive technology

**Status:** Closed
**Resolution:** A focused select opens on Space, Enter, the arrows and F4 and a letter chooses by its label; an open menu takes the arrows, Home, End, PageUp, PageDown, Enter, Space and letters, closes on Escape, Tab and system Back, and is held to the window and scrolls (`presenter/select.rs`); its accessibility and its presentation as Android's and Windows' platform popup are `issues/20261111-select-menu-reaches-no-assistive-technology.md`.
**Systems:** host/linux/src/presenter/control.rs, host/linux/src/presenter/typing.rs, host/linux/src/presenter/back.rs, host/linux/src/presenter/access.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. The presenter in `host/linux/src/presenter/`
is the one Linux, Windows and Android link.

A press on a `select` sets `self.menu` (`host/linux/src/presenter/control.rs:131`) and
the presenter paints a menu of its own over the frame (`menu_geometry`,
`control.rs:356`). That menu is no surface of the top layer, so nothing that serves
those surfaces serves it:

- **Keys.** `key_pressed` (`presenter/typing.rs:649`) asks the top layer's surfaces,
  the shortcuts and Back, never `self.menu`, and `key_default` (line 718) has no
  branch for a select, open or closed.
- **Back.** `system_back` (`presenter/back.rs:227`) closes a top-layer surface, then
  goes to the route. `back_goes` (line 188) does not count the menu.
- **Height.** `menu_geometry` gives every option a row with no limit
  (`control.rs:378`) and puts a menu taller than the window at `y = 0` (line 382).
  The painter and `menu_press` (line 410) share that arithmetic and nothing scrolls it.
- **Accessibility.** A select is one leaf node (`presenter/access.rs:675`, 797),
  `access_tree` (line 1129) never reads `self.menu`, and `access_act` (line 1431) has
  no action that sets a dropdown's value.

Only a pointer press, a value set (`control.rs:279`) and a replaced runner
(`presenter.rs:1404`) close the menu.

## The failure

A `select value=pick change=choose` with options `a`, `b`, `c`:

| Interaction | Observed | Expected |
| --- | --- | --- |
| Closed and focused (`tabindex=0`): Space, Enter, ArrowDown, ArrowUp, `b`, Home, End | the menu stays closed and `pick` stays `a` | Space, Enter and ArrowDown open it; a letter chooses by its first character |
| Open: ArrowDown, Enter, `c` | nothing | the arrows move, Enter chooses, a letter chooses |
| Open: Escape | stays open | closes |
| Open: Tab | the focus moves to another control and the menu stays painted over the page | closes |
| Open, first screen: system Back | `back_goes()` and `system_back()` answer false, which tells the shell to leave the app; the menu is still open | Back closes the menu (LLP 1115.001 D6, step 2) |
| 60 options in a 300 by 400 window, the select at the top or at the bottom | the menu is `(0, 0, 138, 1628)` with 27-point rows: 14 whole rows show; a wheel, ArrowDown, End and PageDown leave it there | every option can be chosen: the list is held to the window and scrolls |
| An assistive technology presses the select | the menu opens and no new statement is told: the tree is still the dropdown and the page's nodes, the button after it still told as pressable; the dropdown has neither `EXPANDABLE` nor `EXPANDED` | the options as nodes, the chosen one selected, the page under them left out, the dropdown expanded |
| Tab, or a press | the select is no Tab stop and a press does not focus it | `issues/20261110-painted-host-tab-skips-select-range-date-file.md` |

Expected: the WAI-ARIA Authoring Practices' select-only combobox gives the keys.
HTML-AAM maps a `select` to a combobox whose popup holds its options. WinUI's
`ComboBox` and Android's `Spinner` and Material's exposed dropdown menu each bound
the list to the screen, scroll it, close on Escape or Back and expose their items.

## Verification

Temporary tests in `exact-linux`'s library booted each form with the CPU painter and
drove `key_down`, `pointer_down`, `pointer_up`, `wheel_at`, `system_back`,
`reader_access` and `access_act`. Every row above is their printed output; for
example `menu after Escape: Some(6)`, `back_goes=false took=false`, and for the long
menu a press on row 13, the last whole row, chose `v13`, while the agent's `type`
still chose `v59`. Not run on a device, a display, or under TalkBack or Narrator.

## Direction

On Android and Windows open a select's menu as the top layer's menu, which takes keys
(`surface_key`), closes on Escape and Back (`dismiss_surface`), tells its rows
(`presenter/access/surface.rs`) and, on the desktop, scrolls
(`presenter/top/look/menu.rs`). Desktop Linux presents no menu popover, so there the
painted menu needs the bound, the scroll, the keys and the dismissal itself. Give a
closed select its keys in `key_default`.

## Related

The menu is recorded as owed, not new. LLP 1069.001 D7 designs "LLP 1021's
light-dismiss popover" and the keys ("arrows … move within a select, Enter commits");
its as-built for slice 2 says "Not LLP 1021's popover: the menu's rows are not nodes,
so it is the host's". LLP 1115.001's as-built lists "a `select`'s menu" as short of
its D10 on Android, and LLP 1115.004 D10.4 plans "a Fluent `ComboBox` list" for
Windows in stage S2. Nothing in `QUEUE.md` or `issues/` tracks any of it, which is the
only reason for this issue. `QUEUE.md`, "The neutral look on Linux and Windows, what
is owed", item 7 is about the menu's row height.
