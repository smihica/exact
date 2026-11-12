# On Linux and Windows `input type="number"` is a plain text field: the arrows do not step and letters are accepted

**Status:** Closed
**Resolution:** A painted number field steps on ArrowUp and ArrowDown by HTML's `stepUp()` rule within `min` and `max`, with `input` then `change` for each step, and keeps only what a number's editor takes from typed, pasted and driven text (`presenter/number.rs`, the rules of `NumberStepping.swift`); it draws no stepper and its accessibility node is still a plain edit field.
**Systems:** host/linux/src/presenter/field.rs, docs/recipes/number-field.md, docs/recipes/grouped-form.md
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`field_key` (`host/linux/src/presenter/field.rs:989`) edits every text input the same
way. It never reads the field's `type`, `min`, `max` or `step`: ArrowUp and ArrowDown
go to `move_field`, where `moved` answers `None` for them in a single-line input
(`field.rs:161` to 186), and any single character is inserted (line 1071). The agent's
`type` is not filtered either. The field's accessibility node is a plain edit field
with no range.

This painted field is what desktop Linux shows, and what Windows shows by default
(LLP 1115.004's D7 record of 2026-10-30: "D7's painted fallback remains the default").
Android's reader hosts the field in an `EditText` with a number keyboard
(`presenter/hosted.rs:153`) and does not take this path.

The web host's element runs HTML's number state, and the Apple hosts were given it by
`issues/closed/20261009-macos-number-field-behavior.md`
(`host/apple/Sources/ExactKit/NumberStepping.swift`). The recipes describe it and name
no host: "the platform draws its stepper, which adds `step`"
(`docs/recipes/grouped-form.md`; `docs/recipes/number-field.md` says the same). Shelf,
one of the Windows host's first consumers (`rules/DEFERRED.md`), has such a field
(`apps/shelf/app.contract:632`), and LLP 1115.004 §3 names "a number field" among
stage S1's controls.

## The failure

```
input type="number" testId="port" value=port input=edit min=1 max=65535 step=1 width=120
```

focused, `port` at `443`:

| Input | Painted presenter | Chrome, as the closed issue measured it |
| --- | --- | --- |
| ArrowUp | `443` | `444` |
| key `a` | `443a` | unchanged |
| `type port 4a4` | `4a4` | `44` |
| emptied, then ArrowUp | empty | `1` |

Expected: Chrome's column. HTML §4.10.5.1.12 (Number state): the value is a valid
floating-point number or empty, and the arrows step by `step` within `min` and `max`.
WinUI's `NumberBox` also steps on the Up and Down keys.

## Verification

A temporary test in `exact-linux`'s library booted the form with the CPU painter,
focused the field and sent the keys through `key_down` and the text through
`type_text`. It printed `("ArrowUp", "443"), ("ArrowDown", "443"), ("a", "443a"),
("type 4a4", "4a4"), ("empty, ArrowUp", ""), ("65535, ArrowUp", "65535")`, and the
field's accessibility node as role 13 (an edit field) with no range. Chrome was not
run in this review, and neither was Windows or Android.

## Direction

In `field_key`, for a field whose `type` is `number`: step on ArrowUp and ArrowDown
by HTML's rule, one `input` a step, and refuse typed or pasted text as
`NumberStepping.insertion` does. That rule is Swift today, so the painted hosts need
it in Rust.

## Related

`issues/closed/20261009-macos-number-field-behavior.md` (P2) fixed macOS and iOS and
lists Linux as not tested. `QUEUE.md`, "Form controls, what is left" records that
`number` is among the "input types the kernel has no control for", which "still lower
as text fields while the page makes the real widget"; that entry predates the Apple
fix and names no host. Nothing tracks the painted hosts.
