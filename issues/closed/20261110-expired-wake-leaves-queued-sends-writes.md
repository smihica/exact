# An expired wake drops its queued sends but leaves their writes showing and their stall standing

**Status:** Closed
**Resolution:** Expiring a wake removes its queued writes, clears stalls belonging to dropped heads, and publishes the updated overlay and pending state.
**Systems:** `runner/src/runner/queue.rs`, `runner/src/runner/writes.rs`, `runner/src/runner/modules/wakes.rs`
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

When a wake expires, `expire_wake` drops its waiting sends with `drop_queued_of`
(`runner/src/runner/modules/wakes.rs:552`, `runner/src/runner/queue.rs:320-334`), which
removes them from the queue and nothing else. Two things a waiting send holds outlive
it.

**Its write record.** A send to a mutation that declares `refreshes` makes a record
when it is accepted (`runner/src/runner/writes.rs:88-105`), and a pending record that
ends without landing is to be removed (`writes.rs:8-9`). The dropped send's record
stays, so the overlay keeps showing a write that is never sent. A queue's next request
takes the oldest pending record as its own (`writes.rs:109-122`, read at
`runner/src/runner/commit.rs:944-947`): from then on each reply lands the record of
the send before it, the overlay is handed one send's arguments with another's reply,
and one write shows pending for the session. The other path that drops a waiting send
ends its record (`queue.rs:231-232`).

**Its stall.** If the dropped send was a head that a refused `next` stalled,
`stalled[m]` stays set; `forget_waiting` clears it (`queue.rs:342-343`). A later send
waits, with `pending(m)` true, until some commit changes a slot, a resource or a
derive.

## The failure

```
mutation pulled as shape Pulled queue refreshes rows
action ebbed(tag: string)        # the wake's handler
  send pulled = pull(tag)
action person
  send pulled = pull("person")
```

The overlay shows one more row for each pending write. Run `person`, raise the wake
(its send waits), let its 5 s budget pass, answer the request; then run `person` again
and answer it.

```
the first reply landed:  server rows 1; view rows "2"; writes [(4, pending)]; queued []
the second reply landed: server rows 2; view rows "3"; writes [(7, pending)];
                         overlay handed #4 args ["w"] with the second reply
```

Expected: rows 1, then 2, and no write left.

The stall: run `person`, raise the wake, answer the request, and have `rows` refuse
when the wake's send is asked at its `next` (`pulled next refused (…); waits for a
change`). Let the wake expire, then run `person`.

```
five seconds later: requests out 0; queued [(pulled, 1)]; pending(pulled) true
```

Expected: the send is asked at once, as it is when its action also writes a slot.

## Verification

Temporary tests in `contract/cli/tests/it/`, over the `tests/tide` fixture's wake and a
Rust `DataSource`, ran both sequences and printed the lines above. A wake whose
handler sends twice with nothing else in flight leaves the same orphan. Not run on a
device.

## Direction

Carry each waiting send's write id on `queue::Waiting`. Have `drop_queued_of` end
those records, clear the stall of a mutation whose head it dropped, and make the
commit that shows it.

## Related

`QUEUE.md`, "One-slot invariants of a queue mutation's slot" and "An answer's storage
step whose 30 s wait runs out lets the queue move on while it may still land" concern
the slot and a storage step. No issue names `drop_queued_of`.
