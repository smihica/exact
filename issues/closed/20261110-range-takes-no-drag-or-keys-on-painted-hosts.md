# A range on the painted hosts cannot be dragged and takes no key

**Status:** Closed
**Resolution:** A contact on an enabled range holds it until the lift, moving the thumb with an `input` for each change and a `change` on the lift (a finger once it moves along the track), and a focused range steps on the arrows, Home, End, PageUp and PageDown as Chromium's does (`presenter/range.rs`); whether Tab reaches a range stays with `issues/20261110-painted-host-tab-skips-select-range-date-file.md`.
**Systems:** host/linux/src/presenter/contact.rs, host/linux/src/presenter/control.rs, host/linux/src/presenter/typing.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. The presenter in `host/linux/src/presenter/`
is the one Linux, Windows and Android link.

The only path from a pointer to a range's value is `press_range`
(`host/linux/src/presenter/control.rs:330`), called from the press a lift makes
(`surface_controls.rs:826`). A range offers no drag candidate (`candidates`,
`presenter/contact.rs:298`), so nothing reads the pointer while it moves. A mouse's
contact is kept past the slop and presses where it lifts (`contact.rs:917` to 922); a
finger's is dropped past the slop (`contact.rs:923` to 929), so its lift presses
nothing.

`key_default` (`presenter/typing.rs:718`) has no branch for a range, so no key reaches
`set_control_value`. A press, the agent's `type` and an assistive technology's step
(`presenter/access.rs:1477`) are the only ways to set one.

## The failure

```
input type="range" min=0 max=10 step=1 value=level input=slide change=settle
```

in a 300-point column, `level` 5, the thumb at the middle:

| Interaction | Observed | Expected |
| --- | --- | --- |
| Mouse: down on the thumb, four moves of 20 points right, up there | `level` is 5 through the moves and 8 after the up; one `input`, one `change` | 6, 7, 8 as the pointer crosses each step, an `input` for each; `change` on the up |
| Mouse: the same down, released 120 points below the range | `level` stays 5; no event | the thumb follows until the up, wherever the pointer is |
| Finger: the same drag | `level` stays 5; no event | as the mouse |
| Focused (`tabindex=0`): ArrowRight, ArrowUp, ArrowLeft, ArrowDown, Home, End, PageUp, PageDown | `level` stays 5 after each | a step up for ArrowRight and ArrowUp, down for ArrowLeft and ArrowDown; Home 0, End 10; a larger step for PageUp and PageDown |
| Tab, or a press | the range is no Tab stop and a press does not focus it | `issues/20261110-painted-host-tab-skips-select-range-date-file.md` |

A finger's tap and an assistive technology's step forward do set it (7 and 6 here).

Expected: HTML fires `input` as a range's value changes and `change` when the person
commits it, and Chrome steps a focused range with these keys. The WAI-ARIA Authoring
Practices' slider pattern gives the same keys. Material's and WinUI's `Slider` follow
the finger or the pointer. LLP 1069.001 D7 decides both for these hosts: "a 16 px
thumb that follows `pan`" and "arrows step a range".

## Verification

Temporary tests in `exact-linux`'s library booted the form with the CPU painter and
drove `pointer_down`, `pointer_move`, `pointer_up` and `key_down`, with
`set_pointer_kind(PointerKind::Touch)` for the finger, as Android's reader sets it
(`canvas/input.rs:44`). They printed: mouse drag, `5` during and `input:8 change:8`
after; released below the range, `5` and no event; finger drag, `5` and no event;
finger tap, `input:7 change:7`; each of the eight keys, `5`. Not run on a device or a
display.

## Direction

Give an enabled range its own contact: the value at the pointer on the down and on
each move (`input` only), `change` on the lift, the contact held until the lift. In
`key_default`, step a focused range through `Range::sanitize`, as `access_act` does
for an assistive step.

## Related

The drag is recorded, not new: LLP 1069.001's as-built for slice 3 says "A press puts
the thumb under the pointer (`input` then `change`). Dragging the thumb is not built."
D7 decides the keys and no as-built says they were built. Neither is in `QUEUE.md` or
`issues/`, which is the only reason for this issue; the presenter has since become
Android's (LLP 1115.001 D9) and Windows's (LLP 1115.004 D12), whose sliders it paints.
