# A refused `type <id> key Shift` leaves the modifier held for every later tap and key

**Status:** Closed
**Resolution:** Refused key-downs preserve prior physical modifier holds and release only newly accepted chord modifiers.
**Systems:** host/linux/src/agent.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The agent's `type … key` marks the key's own modifier bit before it delivers the key
(`host/linux/src/agent.rs:573-581`):

```rust
p.hold_modifier(code, !up);
let r = p.type_key(id, code, logical, !up, repeat);
if phase.is_none() && r.is_ok() {
    p.hold_modifier(code, false);
    let _ = p.type_key(id, code, logical, false, false);
}
if phase.as_deref() != Some("down") || (r.is_err() && !repeat) {
    release(p, &modifiers);
}
```

When `type_key` refuses the down (`no view N`, `view N is disabled or inert`,
`view N is inert under the open surface`; `presenter/typing.rs:306-345`), the bit is
cleared only for a chord's leading modifiers, which `release` walks. For a key that is
itself a modifier (`Shift`, `Control`, `Alt`, `Meta` and their `Left` and `Right`
codes), `chord_modifiers` is empty (`agent.rs:1347-1363`), so nothing clears it.
`Presenter::modifiers()` then reports it held until the process exits or a later
accepted `key` of the same code releases it.

The Windows host and the Android reader run this agent too (`agent.rs:23-24`).

## The failure

```
button press=pick testId="b"            # pick logs e.shiftKey
button disabled=true testId="off"
```

```
tap b                  -> log "false;"
type off key Shift     -> {"error":"view 3 is disabled or inert"}
tap b                  -> log "false;true;"
```

Expected `false;false;`: a key the host refused was never pressed. Every later press
carries `shiftKey: true` (key events read the same `modifiers()`), so a drive that
goes on after the error exercises shift-click paths without asking for them.

## Verification

A temporary test in `agent.rs` booted that plan and sent the requests through `handle`.
It printed `type 3 key Shift -> {"error":"view 3 is disabled or inert"}`,
`modifiers after the refusal: shift=true` and `log: Some("false;true;")`.

The same test with `"phase":"down"` left Shift held, and `Control`, `Alt`, `Meta` and
`ShiftRight` each left their own bit held. Two controls behaved: an accepted
`type b key Shift` left nothing held, and a refused chord `Shift+a` released its
Shift.

Not run: the Windows and Android carriers.

## Direction

Restore the key's previous held state when its down is refused. A refused chord
releases only modifiers newly pressed and accepted during that request; it leaves
earlier holds intact and sends no key-up for a refused down.

## Related

`issues/closed/20261009-capture-phase-key-events.md` added the held-modifier state and
`issues/closed/20261007-cross-browser-modifier-keys.md` is the web drivers' chord
order; neither covers a refused key.
