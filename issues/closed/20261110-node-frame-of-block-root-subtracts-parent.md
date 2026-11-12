# The agent's `node` reports a popover's, sheet's or pane's `frame` less its parent's, a difference between two spaces

**Status:** Closed
**Resolution:** The node and frames replies share a calculation that keeps containing-block roots in their own block's space.
**Systems:** runner/src/agent.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A node with a containing block of its own (a popover in the top layer, a sheet's route, a split's pane) is laid out as a root: "Its frames are in the block's own space" (`kernel/src/kernel/block.rs:13-14`). Its parent's frame is in the page's space.

The agent answers a node's frame in its parent in two places. `frames` knows the rule (`runner/src/agent.rs:177-182`):

```rust
// A block's root is in its block's space, not its parent's.
let (px, py) = node
    .parent
    .filter(|_| kernel.host_block(id).is_none())
```

`node` does not (`runner/src/agent.rs:613-616`): it subtracts the parent's frame from every node's. For a block's root the result is a difference between two spaces and means nothing. The driver prints it for `layout <target>` as `frame X,Y W×H (kernel, in the parent)` (`scripts/agent-inspect.mjs:428`), and LLP 1012 calls it "the kernel's `frame` in the parent".

## The failure

The tree of `a_blocks_root_is_answered_in_its_block` (`runner/tests/it/frames_agent.rs:115`): node 3 sits at y 50 in the page; its children are node 4, given a host block, and node 5, a popover. Asked `{"op":"frames"}`, then `{"op":"node","id":N}`:

```
frames: [3,1,0,50,100,50,0],[4,3,0,0,80,40,0],[5,3,0,0,300,0,0]
node 3: "frame":{"x":0,"y":50,"w":100,"h":50},"absolute":{"x":0,"y":50,"w":100,"h":50}
node 4: "frame":{"x":0,"y":-50,"w":80,"h":40},"absolute":{"x":0,"y":0,"w":80,"h":40}
node 5: "frame":{"x":0,"y":-50,"w":300,"h":0},"absolute":{"x":0,"y":0,"w":300,"h":0}
```

Expected: `node`'s `frame` for 4 and 5 is at `0, 0`, as `frames` answers and as that test's message says ("each at its block's origin, not 50 above it"). In an app the error is the parent's offset in the page, so it grows with how far down the page the popover's or sheet's parent sits.

## Verification

A temporary test built that tree, asked `agent::handle` the three `node` requests, and printed the lines above (`cargo test -p exact-runner --test it <name> -- --nocapture`). Not run through a host's `layout <target>`; the Linux and Apple hosts answer it by sending this `node` request to `agent::handle` (`host/linux/src/presenter.rs:1189`, `host/apple/Sources/ExactKit/AgentNative.swift:175`).

## Direction

Give `node` the filter `frames` has, from one shared helper so the two cannot drift again.

## Related

No issue or QUEUE entry names `node`'s frame. The `frames` half was fixed with its test; this is the copy that was missed.
