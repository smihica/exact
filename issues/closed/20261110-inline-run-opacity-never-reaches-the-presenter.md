# An inline run's `opacity`, and any opacity animation or transition on it, never reaches the Apple presenter

**Status:** Closed
**Resolution:** The `paragraph` op carries each run's presented opacity times its enclosing runs' (the engine samples a run's animations, and a frame that moves the value paints the paragraph again), and the presenter multiplies it into the run's ink, background, shadow and stroke.
**Systems:** host/apple/src/presence.rs, host/apple/src/paragraph.rs, host/apple/src/svg.rs, host/apple/Sources/ExactCore/BatchShapes.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A `text` inside a `text` is an inline run: it has no view, and its paragraph paints it from the `paragraph` op's `runs` (`host/apple/src/paragraph.rs:209-302`, commit `dd6f5dcd`). Opacity has no way to get there:

- A style dictionary never carries `opacity` (`host/apple/src/style.rs:137`); a view gets it from `present` ops.
- `present_value` drops every presented value of a run (`host/apple/src/presence.rs:462`: `if self.inline_runs.contains_key(&view) || self.svg.presented(view) { return; }`), and both presenters apply a `present` only to a view (`Mac/PresenterMac.swift:1200`, `IOS/PresenterIOS.swift:994`).
- The run's entry is its props and its inherited style (`paragraph.rs:237`, `:266`), and Swift's run style has no opacity key (`host/apple/Sources/ExactCore/BatchShapes.swift:219-221`).

So a run with `opacity` paints at full strength, and a transition or animation of it runs in the engine and shows nothing. The compiler accepts each of them on a run without a warning.

## The failure

```contract
component Hello
  view
    column padding=12
      text font-size=24 font-weight=700
        text "Posted "
        text "yesterday" opacity=0.3
```

Expected: "yesterday" at 30% opacity, as the web build draws it and as a browser draws a `<span style="opacity: .3">` in a `<p>` (CSS Color 3 §3.2: `opacity` applies to all elements). Observed on macOS: both words in full ink. With `animation="pulse 1000ms linear infinite alternate"` (opacity 1 to 0.2) or `transition="opacity 1s linear" opacity=(on ? 0.2 : 1)` on the run, nothing moves.

## Verification

With the plan above as `apps/hello/app.contract`, the darkest pixel of each word in a screenshot (0 is black):

- macOS (`bun host/apple/build.mjs hello-apple`, `bun scripts/agent.mjs macos --app hello "screenshot out.png"`): 0 and 0.
- Web (`bun host/web/build.mjs hello`, the same drive with `web`, Chrome 156): 0 and 177, which is black at 30% over white.

On macOS a run with the animation above and one with the transition, pressed at 500 ms, stayed in full ink at 500, 1000 and 1250 ms. A paragraph with the same animation read opacity 0.6, 0.2 and 0.4, and one with the transition 1, 0.6 and 0.4.

A temporary test in `host/apple/src` (`cargo test -p exact-apple --lib <name> -- --nocapture`) printed the ops that name the run. For `opacity=0.3` the run's entry in the `paragraph` op is `"style":{"text_color":…}` and no `present` follows. For the transition, 500 ms after the press the engine holds 0.6 and the batch has no op for the run or its paragraph. For the animation, an `animations` op with an `opacity` spec goes to the run's id, which has no view (`SvgScene.swift:654` returns).

iOS and tvOS are by reading: the same Rust, and the same run style.

## Direction

Carry a run's presented opacity on the run in the `paragraph` op, as its moving `color` already is (`paragraph.rs:263`, LLP 1062 D5), and multiply it into the run's ink where the paragraph paints. Keep a run out of `SvgState::boxes` (`host/apple/src/svg.rs:143`).

## Related

`issues/closed/20261030-a-node-mounted-at-one-opacity.md` is about views. `host/linux/src/paint/inline.rs` has no run opacity either (by reading, not run). Not in LLP 1001's deviations, `rules/DEFERRED.md`, QUEUE or the pitfalls.
