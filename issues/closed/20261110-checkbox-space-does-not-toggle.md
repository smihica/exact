# Space does not toggle a focused checkbox or switch on the painted hosts

**Status:** Closed
**Resolution:** Space at a focused, enabled checkbox or switch runs a click's path, its `press` and then the toggle with `input` and `change`, and Enter still toggles neither.
**Systems:** host/linux/src/presenter/typing.rs, host/linux/src/presenter/control.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. The presenter in `host/linux/src/presenter/`
is the one Linux, Windows and Android link.

`key_default` (`host/linux/src/presenter/typing.rs:718`) gives Space and Enter a
default action for a button and for a node with a `press` handler (lines 746 to 761),
and Space and the arrows for a radio (`radio_key`, line 763). Any other node falls
through to `field_key` (line 771), which returns unless it is a text field. Nothing on
the keyboard path calls `toggle_control` (`presenter/control.rs:40`), which a click
and a TV remote's Select (`presenter/remote.rs:207`) call.

A checkbox and a switch are Tab stops (`surface_controls.rs:710`), so with a keyboard
a person reaches one and cannot change it.

## The failure

Each control is focused, then sent the key:

| Control | Key | Observed | Expected |
| --- | --- | --- | --- |
| `input type="checkbox" checked=on input=saw change=flip` | Space | `on` stays `false`; no `input`, no `change` | `on` is `true`; `input` then `change`, as a click sends them |
| `input type="checkbox" switch checked=sw change=flick`, and the same with `role="switch"` | Space | stays off | on |
| `input type="checkbox" checked=on change=flip press=pressed` | Space | `pressed` runs; `on` stays `false` | `pressed`, then the toggle with `input` and `change`, as a click |
| `input type="checkbox"` with no `checked` | Space | stays unchecked | checked |

Enter toggles neither the checkbox nor the switch, as in Chrome.

Expected: Chrome toggles a focused checkbox on Space. The WAI-ARIA Authoring
Practices' checkbox and switch patterns give Space. WinUI's `CheckBox` and
`ToggleSwitch` toggle on Space, and Android's `CheckBox` and `Switch` take Space and
Enter as a click (`KeyEvent.isConfirmKey`). LLP 1069.001 D7 decides it for these
hosts: "Keyboard: Space toggles".

## Verification

Temporary tests in `exact-linux`'s library booted each form with the CPU painter,
focused the control with `set_focus` and sent the key through `key_down`, the entry a
display's keyboard uses. Tab reached the checkbox and the switch. After Space, and
after Enter, the bound checkbox was `on=false` with no event recorded and the switch
`false`; with a `press` handler, `on=false presses=1`; the unbound checkbox kept no
state. A click on each then toggled it (`on=true`, `input:true change:true`). Not run
on a Windows or Linux display.

## Direction

In `key_default`, before the pressable branch, Space on an enabled checkbox or switch
calls `toggle_control`, as `remote_select` does, so the key and the click share one
path.

## Related

This gap is recorded, not new: LLP 1069.001's as-built note for slice 1 lists
"keyboard Space on Linux" under "Not built in this slice", adding "no Linux control
takes focus". Checkboxes and switches take the focus now, and the presenter has become
Windows's and Android's. Nothing in `QUEUE.md` or `issues/` tracks it, which is the
only reason for this issue.
