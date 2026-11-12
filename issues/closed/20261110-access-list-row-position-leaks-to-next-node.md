# The accessibility node after a virtual list is told it is one of the list's rows

**Status:** Closed
**Resolution:** Accessibility set positions stay in their declaring subtree, including rows clipped out of virtual lists.
**Systems:** host/linux/src/presenter/access.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. `host/linux/src/presenter/access.rs` builds
the accessibility tree that Android's reader and the Windows host read.

`access_walk_node` records a node's `aria-posinset` and `aria-setsize` in the build's
`item` slot before it knows whether a node will be emitted for it (line 775):

```rust
if let Some(at) = position(PropId::AccessibilityPosInSet) {
    let size = position(PropId::AccessibilitySetSize).unwrap_or(0);
    b.item = (at as i32, size as i32);
}
```

`access_emit` takes the slot only when it pushes a node
(`set: std::mem::take(&mut b.item)`, line 617), and it returns earlier for a box that
does not show in its scroller's clip (line 597). The runner publishes both props on
every virtual list row's wrapper (`runner/src/instance/collection/views.rs:262`), and
a list keeps rows mounted past what shows. Those rows emit nothing, so the last one's
place stays in the slot and goes to the next node emitted anywhere after the list.

Windows sets UIA's position in set and size of set from that pair
(`host/windows/src/accessibility.rs:421`). Android's reader gives any node with a
place a `CollectionItemInfo`, and gives a scroller a `CollectionInfo` when a child has
one (`host/android/canvas/src/main/java/dev/exact/canvas/Access.kt:316` to 322).

## The failure

```
component App
  resource rows = rows() as shape number[]
  state picked = -1
  action pick(n: number)
    picked = n
  view
    column width=320 height=480
      list virtualized=true width=320 height=400
        for x in rows key=x
          button press=pick(x) height=40 width=300
            text `Row ${x}`
      button press=pick(-1) height=40
        text "Done"
      text "Footer" height=20
```

With 400 rows and the list at its top, `access_tree()` answers ten row buttons with
`set` `(1, 400)` to `(10, 400)` under the list, then the button "Done" with parent 0
and `set == (20, 400)`: the place of the last mounted row, which is below the list's
port. Expected `(0, 0)`: "Done" is in no set.

## Verification

A temporary test in `exact-linux`'s library booted this app with the CPU painter and
printed every node of `access_tree()`. It printed the ten rows, then
`role=3 parent=0 set=(20, 400) name="Done"` and
`role=1 parent=0 set=(0, 0) name="Footer"`. Not run under TalkBack or Narrator; what
they are given is read from `Access.kt` and `accessibility.rs`.

## Direction

Scope the place to the subtree that declared it: save `b.item` on entering a node
that sets it and restore the saved value after its children are walked, so a place no
node took is dropped.

## Related

`QUEUE.md`, "Native table accessibility stops at roles" is about a table's cells, not
a list's `aria-posinset`. `access_tests.rs` checks a row's own `set` (`(4, 400)`) and
has no node after a list.
