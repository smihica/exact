# A block box mounted with only paint rows lays its children out by `order`

**Status:** Closed
**Resolution:** Child ordering reads the arena's current computed display, so newly mounted block containers keep document order even with only paint styles.
**Systems:** kernel/src/layout/order.rs, kernel/src/txn.rs, kernel/src/layout.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS `order` applies to flex items and grid items and does nothing in a block container
(CSS Flexbox §5.4). LLP 1001 says the same of the kernel: "A flex or grid container
hands the layout engine its children in order-modified document order".
`order::laid_out` decides whether the parent is such a container from the engine's
copy of its style (`kernel/src/layout/order.rs:35-37`), not from the arena.

Inside a batch that copy can be stale. A node the batch creates and then styles is
made with the engine's default style (`kernel/src/txn.rs:662-663`,
`kernel/src/layout.rs:261-268`), whose `display` is `flex`
(`vendor/taffy/src/style/mod.rs:234`). Its real style is derived at its `SetStyle`
only when that changes a layout row (`txn.rs:822-824`), otherwise at the end of the
batch (`txn.rs:978-982`). A `SetChildren` in between sees a flex container and sorts
the children by `order`. The restyle at the end of the batch makes the box a block
and does not sync its children again, so the sorted order stays.

The runner emits a node's `SetStyle` before its `SetChildren`
(`runner/src/instance.rs:899-911`), so every box mounted with a style that sets no
layout row, such as a background colour alone, is in that state when its children
arrive.

## The failure

```contract
component App
  view
    view background-color="#ffffff"
      view height=10 order=2 testId="a"
      view height=20 order=1 testId="b"
```

Observed after the mount: `a` at y 20, `b` at y 0. Expected: `a` at y 0, `b` at y 10,
which is what Chrome 156 gives the same boxes, what a kernel rehydrated from the same
columns gives, and what this kernel gives when the parent also says `display="block"`.
The incremental layout disagrees with the rebuild that the result-equality gate
(LLP 1001 §5) holds it to.

## Verification

A temporary test compiled the Contract above, booted it in a `Runner` on the
monospace kernel and laid it out at 402 by 874:

```
a booted [0.0, 20.0, 402.0, 10.0], rehydrated [0.0, 0.0, 402.0, 10.0]
b booted [0.0, 0.0, 402.0, 20.0],  rehydrated [0.0, 10.0, 402.0, 20.0]
```

With `display="block"` added to the parent, booted and rehydrated agree (`a` at y 0,
`b` at y 10). The same ops through `Kernel::apply` (create, style, then children)
give the same frames. Headless Chrome for Testing 156.0.8078.4 puts `a` at y 0 and `b`
at y 10 under the block parent, and at y 20 and y 0 under `display: flex;
flex-direction: column`. Not run: a native host.

## Direction

Read the container's `display` from the arena in `order::laid_out`, so the answer does
not depend on when the engine style is derived. Syncing the children again in the
end-of-batch restyle would also work, at a pass per mounted box.

## Related

`issues/` and `QUEUE.md` hold nothing on `order` or on nodes created with the engine's
default style. LLP 1001's declared deviation for `order` is that painting stays in
tree order.
