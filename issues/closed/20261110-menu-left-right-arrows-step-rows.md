# In a painted menu Left and Right step through the rows, and Right does not open a submenu

**Status:** Closed
**Resolution:** Painted menus open a focused row's submenu on Right and focus its first enabled row; Left is inert, and alerts retain horizontal navigation.
**Systems:** host/linux/src/presenter/top.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A key while a top-layer surface is open goes to `surface_key`
(`host/linux/src/presenter/top.rs:727`). Lines 752 and 753 take the arrows in pairs:
`"ArrowDown" | "ArrowRight"` steps to the next row and `"ArrowUp" | "ArrowLeft"` to the
previous one, wrapping. The key events of a shell that makes the menu's window (Android's
reader) do the same (lines 670 and 671). That suits an alert's buttons, which sit in a
row. A menu's rows are a column, so there Left and Right move the highlight up and down,
and Right on a row that opens a submenu (`Row::opens`, `top/shape.rs:48`) moves past it.
Only Enter or Space opens the submenu (`choose`, line 596).

On Windows this menu stands for WinUI's `MenuFlyout` (`top.rs:9`; LLP 1115.004 D10.1, and
D9: "The keyboard is Windows's"). Microsoft's guidance: "Up/Down arrow key moves focus
inside `ListView` and `MenuFlyout`", and Right and Left navigate items "displayed in a
single row" ("Keyboard interactions", Navigation); the Right arrow opens a split menu
item's submenu and "Esc or the Left arrow key" closes it ("Menu flyout and menu bar"). The
WAI-ARIA Authoring Practices give a menu the same keys (Menu and Menubar pattern, Keyboard
Interaction): Right Arrow on an item with a submenu "opens the submenu and places focus on
its first item", and where no menubar is present it "would not do anything" on another
item; Left Arrow in a submenu "closes the submenu and returns focus to the parent
menuitem".

## The failure

The menu of `top_invocation_tests.rs` (`APP`), whose second row opens a submenu:

```
column id="menu" popover="auto" role="menu"
  button press=rename popovertarget="menu" popovertargetaction="hide" testId="menu-rename"
    text "Rename"
  button commandfor="sub" command="show-popover" testId="to-sub"
    text "More"
  button press=retarget commandfor=subName command="show-popover" testId="to-retargeted"
    text "Elsewhere"
  button press=keep commandfor="dpm" command="show-modal" testId="to-confirm"
    text "Ask first"
column id="sub" popover="auto" role="menu"
  button press=keep popovertarget="sub" popovertargetaction="hide" testId="sub-keep"
    text "Keep"
```

With `serve_overlays()` (the Windows overlay), open the menu, press Arrow Down twice to
reach More, then Arrow Right.

Expected: the submenu `sub` opens with its first row focused. Observed: the menu stays
open and the focus moves to Elsewhere. Arrow Left then moves up to More, again to Rename,
and a third time wraps to Ask first.

## Verification

A temporary test in `top_invocation_tests.rs` printed the focused row after each key:
`to-sub` after two Arrow Down, `to-retargeted` after Arrow Right, then `to-sub`,
`menu-rename` and `to-confirm` after three Arrow Left, with the surface `menu` open
throughout. Run on macOS; `surface_key` has no `cfg` on these arms. Not run on Windows or
Android; WinUI's keys are from the two pages quoted.

## Direction

In `surface_key` and `surface_event`, branch on the surface's kind. For a menu, Right
chooses the focused row only when it opens another surface, and focuses that surface's
first row; Left does nothing, since a submenu here replaces its parent and has none to
return to. Keep both arrows stepping for an alert.

## Related

Nothing in `QUEUE.md`, `issues/` or `docs/agent-pitfalls.md` names a menu's arrow keys.
The closed issue `20261009-contract-menu-bar-extensions` covers macOS submenus.
