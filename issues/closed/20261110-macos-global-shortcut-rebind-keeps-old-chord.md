# On macOS, a global shortcut whose `keys` change keeps its old chord registered until the app quits

**Status:** Closed
**Resolution:** The macOS module keeps each `(id, keys)` pair it reads, so a release of a reader's old keys unregisters that chord while its new keys stay registered.
**Systems:** packages/global-shortcuts/apple/GlobalShortcuts.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, the module remembers what a session reads as one chord per `id`: `private var reading: [String: String]` (`packages/global-shortcuts/apple/GlobalShortcuts.swift:22`), written at each read (`reading[id] = keys`, line 33).

When a reader's arguments change, the runner hands over the new arguments before it releases the old group (LLP 1133.009 D10 item 12), and it releases a group only once no request of the source is in flight (`runner/src/runner/modules/bound.rs:370` to `405`). So for a reader that moves from one chord to another under the same `id`:

1. `globalShortcut(id, new)` registers the new chord and overwrites `reading[id]` with it.
2. `release("globalShortcut", args: {id, old})` then fails its guard, `reading[id] == keys` (line 58), and returns without calling `HotKeys.shared.release`.

The old chord's Carbon registration and its reader entry stay. `release(name)` and `destroy` walk `reading` (line 66), which no longer names the old chord, so it is not released when the last reader leaves either.

What the person sees: after choosing a new hot key, the old one still runs the action (it emits `globalshortcut` for that `id`), and the system goes on taking the old chord's key presses for this app until it quits.

The Windows module keeps `(id, keys)` pairs and releases the pair it is given (`packages/global-shortcuts/windows/lib.rs:11`, `74` to `80`).

## The failure

An app that lets the person choose its hot key:

```
resource hotkey = globalShortcut(id="toggle", keys=binding)
```

`binding` changes from `Control+Alt+Meta+F18` to `Control+Alt+Meta+F19`.

Observed: both chords are registered; after the reader goes, F18 still is. Expected: only the chord a reader asks for is registered (LLP 1133.025 D3: `keys` is registered "while something reads it").

## Verification

Compiled the module's file as written on a Mac, with a stand-in for `ExactPackage` and `private` dropped from `HotKeys`, `Chord`, `Reader` and `chords` so the table can be printed, and made the runner's calls in its order; Carbon registered the chords for real:

```swift
m.later("globalShortcut", ["id": "toggle", "keys": old], preferred: nil) { _ in }   // answered
m.later("globalShortcut", ["id": "toggle", "keys": new], preferred: nil) { _ in }   // answered
m.release("globalShortcut", args: ["id": "toggle", "keys": old])
m.release("globalShortcut")
```

The registered chords after the first, third and fourth line:

```
one reader of Control+Alt+Meta+F18: ["Control+Alt+Meta+F18"]
after the reader moved to Control+Alt+Meta+F19: ["Control+Alt+Meta+F18", "Control+Alt+Meta+F19"]
after the last reader left: ["Control+Alt+Meta+F18"]
```

Not driven through the runner in an app.

## Direction

Key `reading` by `id` and `keys` together, as the Windows module does, so a release names the group it was asked for.

## Related

QUEUE "The desktop modules, what is owed" item 2 is the origin of a hot key's press on macOS, not its registration's life. No issue covers it.
