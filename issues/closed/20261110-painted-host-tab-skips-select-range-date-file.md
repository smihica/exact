# On the painted hosts Tab skips a `select`, a range and a file control

**Status:** Closed
**Resolution:** `focusable` admits every control HTML lists as a focusable area (a select, a range, a file control, and a date or time control where its press opens a picker), so Tab and a press reach each, and Space or Enter at a focused file, date or time control clicks it; a range's and a select's own keys are `issues/20261110-range-takes-no-drag-or-keys-on-painted-hosts.md` and `issues/20261110-select-menu-and-keys-on-painted-hosts.md`.
**Systems:** host/linux/src/surface_controls.rs (the painted presenter: Linux, Windows, Android TV's remote)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Presenter::focusable` is documented as "The web's focusable nodes: controls, inputs,
buttons and links". Tab's stops (`presenter/typing.rs:486`, `focus_stop`), a press's focus and
`focus()` read it, and so do a TV remote's stops (`presenter/remote.rs:275`). It admits
four kinds of control (`host/linux/src/surface_controls.rs:710-718`):

```rust
|| matches!(
    exact_kernel::ControlKind::of(n.node_type, n.props),
    Some(ControlKind::Button | ControlKind::Radio | ControlKind::Checkbox | ControlKind::Switch)
)
```

`Select`, `Range`, `File`, `Date`, `Time` and `DateTimeLocal` are left out, and the
handler clause after it counts `focus`, `blur`, `key`, `keyup`, `press` and the
clipboard events, not the `change` or `input` such a control carries. So a person at a
keyboard cannot reach a dropdown, a slider or a file button, which a pointer operates.
A date or time field is skipped too; on a desktop its press does nothing either
(LLP 1069.001: "No calendar, and no keyboard editing yet").

HTML: "Modulo platform conventions, it is suggested that the following elements should
be considered as focusable areas and be sequentially focusable: … input elements whose
type attribute are not in the Hidden state, select elements, …"
(https://html.spec.whatwg.org/multipage/interaction.html#the-tabindex-attribute). The
platforms this presenter paints for keep that list. Windows: "Most focus-capable
controls are included in tab order by default" and "Any UI that can be activated by
pointer should also be invokable by keyboard"
(https://learn.microsoft.com/windows/apps/design/accessibility/keyboard-accessibility),
which LLP 1115.004 D9.1 takes as the rule. Android: "All interactive widgets provided
by the Android framework … are focusable"
(https://developer.android.com/develop/ui/views/touch-and-input/keyboard-input/navigation).
For Linux, LLP 1069.001 D7 gives these controls keys: "arrows step a range or move
within a select".

## The failure

```
component App
  state level = 5
  state pick = "a"
  action moved(v: number)
    level = v
  action picked(v: string)
    pick = v
  view
    column width=400 height=600
      input testId="text" height=30
      input type="range" min=0 max=10 value=level input=moved change=moved testId="range" width=200
      input type="checkbox" testId="check"
      select value=pick change=picked aria-label="Pick" testId="select"
        option "A" value="a"
        option "B" value="b"
      input type="date" testId="date"
      input type="time" testId="time"
      input type="file" testId="file"
      button "Save" testId="save" height=40
```

Tab, nine times. Painted host: `text check save text check save text check save`.
Chrome 156 on the same form in HTML: `text range check select date time file save`.

## Verification

A temporary test in `host/linux/src/presenter/arrive_tests.rs` pressed Tab through
`key_down` (`cargo test -p exact-linux --lib`) and printed the order above and
`focusable(…) = false` for range, select, date, time and file. A second clicked the
range, the select and a date field: the range's value moved from 5 to 2 and the
select's menu opened, each with the focus left on nothing; the date did nothing. The
file control's press was read (`presenter/control.rs:106-113` opens its picker), not
run. Chrome's order is from the form in HTML, driven with the repository's
`playwright-core` and Chromium (156.0.8078.4). Not run on Windows, with a real
keyboard, or with a TV's remote.

## Direction

Admit `Select`, `Range` and `File` in `focusable`, and the date kinds where a press
opens a picker. A focused range then needs the arrow keys, and a focused select or
file control Space and Enter, which `presenter/control.rs` gives a press today.

## Related

`issues/20261110-select-is-tab-stop-with-keyboard-navigation-off.md` is the Mac's
case of the same sentence of HTML: AppKit's convention leaves a pop-up button out with
Keyboard Navigation off, and Exact stops there anyway. Here the platforms' convention
includes these controls and Exact leaves them out.
`issues/closed/20261009-macos-controls-default-tab-order.md` fixed this gap on macOS.
