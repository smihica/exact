# The Apple text engine keeps a font for every size, weight and axis value it was ever asked for

**Status:** Closed
**Resolution:** Each Apple text engine caches at most 256 fonts across all resolution paths and clears the cache on memory pressure.
**Systems:** host/apple/Sources/ExactCore/Text.swift (`TextEngine.fonts`, `font(size:weight:family:italic:…)`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. `TextEngine.fonts` (`host/apple/Sources/ExactCore/Text.swift:518`)
caches each font under a key made of its family, size, weight, slant, stretch,
variation settings and features (`:745`, `:761-762`). Entries are added at `:750`,
`:781`, `:784` and `:793`. The dictionary is emptied only when a plan's font
catalog is installed (`:659`), at a boot or a reload. Memory pressure drops
shaped text and leaves the fonts (`dropCold`, `:526`, from
`host/apple/Sources/ExactKit/Session.swift:556`). A session has two engines, one
painting and one measuring, each with its own dictionary (`:498-502`).

Nothing bounds the dictionary within a session, and three inputs are continuous:

- `-exact-fit="shrink"`: the kernel scales the text by its box width over its
  natural width (`kernel/src/layout/fit.rs:62-68`), so each step of a live window
  resize is a new size;
- a `font-size` computed from state or the viewport:
  `apps/variable-type/app.contract:55`, `:103` set it from the pad's drag and the
  window's size;
- a transition or animation of `font-stretch` or `font-variation-settings`
  (`runner/src/runner/text_axes.rs`), a new value each frame; `font-weight` moves
  in whole numbers, so it adds at most 1,000 per size.

## The failure

```swift
let engine = TextEngine(resolve: { _ in nil })
for i in 0..<10_000 {
    let font = engine.font(size: 14 + CGFloat(i) * 0.005, weight: 400, family: 0, italic: false)
    // one line of text measured with it
}
```

| fonts asked for | kept | footprint | footprint, dictionary emptied every 60 |
|---|---|---|---|
| 300 | 300 | +1.2 MB | +0.3 MB |
| 1,000 | 1,000 | +27.6 MB | +6.0 MB |
| 3,000 | 3,000 | +154.3 MB | +16.1 MB |
| 10,000 | 10,000 | +597.0 MB | +51.9 MB |

With a weight between two hundreds and a `font-stretch` as well (variable-type's
three bindings), each font asked for keeps three: 1,000 asked, 2,984 kept,
+78.3 MB (+9.3 MB emptied); 3,000 asked, 8,954 kept, +236.5 MB (+27.5 MB).

A minute of an animated axis at 60 Hz is 3,600 fonts. Expected: memory bounded by
the fonts on screen, or by a fixed number of recent ones.

## Verification

Four temporary XCTest classes in `host/apple/Tests/ExactKitTests`, each in its
own process, run on macOS (AppKit) with
`bun host/apple/build.mjs --test --only <classes>`: the loop above, the same loop
emptying `engine.fonts` every 60 fonts, and both again with
`weight: 301 + i % 397, stretch: 75 + CGFloat(i % 501) * 0.1`. Each iteration ran
in an autorelease pool; the footprint is `task_vm_info`'s `phys_footprint`. They
printed the figures above. This drives the engine directly: no app was resized or
dragged, and that those inputs reach `font(size:…)` with a new value each time is
from reading the three paths named.

## Direction

Bound the dictionary: drop the least recently used entries past a few hundred, or
empty it when it passes a limit, since a font is cheap to make again.

## Related

`20261110-hosted-field-face-cache-outlives-its-plan.md` is another Apple cache,
keyed by plan. `20261110-calc-handle-tables-never-shrink.md` is the kernel's.
Neither is this dictionary.
