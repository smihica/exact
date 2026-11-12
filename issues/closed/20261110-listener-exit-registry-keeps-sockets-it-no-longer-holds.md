# A listener that is displaced or loses its socket file leaves its entry in the exit registry

**Status:** Closed
**Resolution:** Displaced or missing listener sockets leave the exit registry before their bindings are dropped, without touching replacement sockets.
**Systems:** host/listen (bind.rs, lib.rs)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`bind::publish` records each published socket as `(path, (device, inode))` in `PUBLISHED` (`host/listen/src/bind.rs:268`, `:301-308`, at `dd6f5dcd`). The registry is "the files this process has published and not removed" (`bind.rs:288-291`), and `at_exit` calls `unpublish` for every entry (`bind.rs:294-299`), which removes the file at `path` when its device and inode equal the entry's (`bind.rs:277-286`).

An entry leaves the registry only inside `unpublish`. A listener gives its socket up in two other ways, and neither removes the entry:

- `Loop::displace` drops the `Bound` when a newer listener takes the address over (`host/listen/src/lib.rs:320-324`);
- `Loop::look` sets `self.bound = None` when the path vanished (`lib.rs:336-343`), and calls `displace` when the path now names another process's socket (`lib.rs:345`).

`teardown` unpublishes only a `Bound` it still has (`lib.rs:620-622`). So the registry keeps an entry for a socket this process no longer holds, one more for each displacement, until the process exits.

## The failure

```rust
let first = publish(&path, false, false).unwrap();
let second = publish(&path, true, false).unwrap(); // a newer listener takes over
drop(first);                                       // what Loop::displace does
// PUBLISHED for the path: [first.identity, second.identity]
unpublish(&path, second.identity);                 // the newer listener closes
// PUBLISHED for the path: [first.identity]
```

Expected: the registry holds nothing for the path once neither socket exists.

## Verification

A temporary test in `bind.rs` ran the lines above and printed the registry's entries for the path:

```
first (16777230, 255428534), second (16777230, 255428537); registered after the first was displaced: [(16777230, 255428534), (16777230, 255428537)]
registered after the second closed: [(16777230, 255428534)]
```

What the leftover entry does at exit was not run. By reading, `unpublish` removes whatever socket is at the path if its device and inode equal the entry's, so the entry is harmless unless a later socket at the same path has the first one's inode number. APFS does not hand a freed inode number out again, so this Mac cannot produce that; a filesystem that reuses them would let this process's exit remove another listener's live socket, which LLP 1133.005 D4 rules out ("on close, remove the path only while it is still this listener's socket").

## Direction

Add a `forget(path, identity)` that drops the registry entry without touching the file, and call it where `displace` and `look` let a `Bound` go.

## Related

`QUEUE.md`, "The local listener, what is owed", lists acceptance drives only. `issues/closed/20261102-listener-accepts-group-writable-ancestors.md` and `issues/closed/20261102-listener-publishes-connection-before-send-state.md` are other parts of the crate.
