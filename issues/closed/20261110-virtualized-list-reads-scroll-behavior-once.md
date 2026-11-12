# A virtualized list reads `scroll-behavior` once, so a bound value that changes is ignored

**Status:** Closed
**Resolution:** Virtualized lists refresh scroll-behavior from the latest emitted binding before planning end-follow corrections.
**Systems:** runner/src/instance/collection/mod.rs, runner/src/instance.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Collection::create` evaluates the list's `scroll-behavior` binding and keeps the answer in `Collection::smooth` (`runner/src/instance/collection/mod.rs:373-378`). Nothing writes the field again. `NodeInst::update_children` passes each update's `scrollFollowEnd` and `keep-mounted` to the collection (`runner/src/instance.rs:1234-1257`), but not this row. The kernel's row follows the binding, so after the first change the node's computed `scroll-behavior` and what the list does with it disagree.

`Collection::restore` decides from that field whether a followed end animates (`collection/mod.rs:746`). A list created while its binding said `auto` never animates its follow, and one created `smooth` always does.

The compiler admits the bound value: the source below compiles.

## The failure

```
component App
  state n = 100
  state calm = true
  action grow
    n = n + 1
  action flip
    calm = not calm
  view
    list virtualized=true height=320 estimated-item-height=32 scrollFollowEnd=true scroll-behavior=(calm ? "auto" : "smooth")
      for x in range(n) key=x
        text `${x}` height=32
```

Report the port at its end (offset 2880, each row measured at 32), run `grow`, report the port at 2912, run `flip`, report again, run `grow`.

Observed: after `flip` the kernel's row is `smooth`, and the second follow's correction (to 2944) has `smooth: false`. Started with `calm = false`, the row after `flip` is `auto` and the correction has `smooth: true`.

Expected: the correction follows the computed value when the scroll happens, as CSS Overflow 3's `scroll-behavior` does, and as LLP 1070.000 §11 says of "a `scroll-behavior: smooth` list's end-follow correction".

## Verification

A temporary test in `runner/tests/it` booted the source with `contract::compile` and the monospace kernel, drove it as above through `Runner::collection_feedback` and `Runner::act`, and printed the list node's computed `scroll-behavior` and each correction as (offset, scroll sequence, smooth):

```
created with calm=true: row Enum("auto"), follow (2912.0, 3, false); after flip: row Enum("smooth"), follow (2944.0, 6, false)
created with calm=false: row Enum("smooth"), follow (2912.0, 3, true); after flip: row Enum("auto"), follow (2944.0, 6, true)
```

Not run: the JS target's list (`host/web-js/list.js`) and a native host.

## Direction

Read the row where the correction is made, from the node's last emitted value, or pass it to the collection's update with `follow` and `keep`.

## Related

`QUEUE.md`'s "Smooth collection jumps cross unbuilt rows" is about what a smooth jump shows, not which lists animate. No issue or `docs/agent-pitfalls.md` entry names a bound `scroll-behavior`. `scroll-restoration`, which the list also reads once (`collection/mod.rs:454-459`), cannot change: the compiler refuses any value but a literal `auto` or `manual` (`contract/lower/src/collection.rs:306`, `lower-attr-value`).
