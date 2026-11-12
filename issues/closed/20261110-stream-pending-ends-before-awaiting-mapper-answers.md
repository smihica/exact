# A stream stops being pending when its first message arrives, before an awaiting mapper has answered

**Status:** Closed
**Resolution:** A stream stays pending and outstanding until its first mapped value commits.
**Systems:** `runner/src/runner/commit.rs`, `runner/src/runner/mapper_turns.rs`, `runner/src/runner/outstanding.rs`
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1016.000 D1 makes each message of a stream one commit that sets the resource's
value, and the first ends `pending`. Its amendment of 2026-11-10 lets a mapper answer
with a promise: "the call's value commits as the stream's message". In
`docs/reference.md:1321-1322`, "the message commits when it settles".

The runner splits the two. `message()` counts the message and syncs the pending flags
as it arrives (`runner/src/runner/commit.rs:1256-1264`; a stream is in flight while
`messages == 0`, `runner/src/runner.rs:301-303`). When the mapper answers with a
promise, that commit only sends the mapper's next turn and updates the view
(`commit.rs:1392-1402`); the value lands later, in `run_turn`
(`runner/src/runner/mapper_turns.rs:178-203`). Between the two commits:

- the view reads `pending(resource)` false over the placeholder value;
- nothing settled, so a derive or a resource argument that reads `pending(resource)`
  still holds true: one frame shows both;
- `outstanding()` lists no stream (`runner/src/runner/outstanding.rs:85-101`), so a
  host records time to interactive before the first value shows
  (`host/linux/src/display.rs:461`).

The web JS target ends `pending` and lands the value in one step, when the promise
settles (`host/web-js/ts-stream.js:191-192`, `host/web-js/rt.js:407-413`).

## The failure

```
component App
  resource feed = feed() as shape string[]
  derive waiting = pending(feed)
  view
    column
      text testId="direct" pending(feed) ? "connecting" : "live"
      text testId="derived" waiting ? "connecting" : "live"
      text testId="count" feed.length == 0 ? "No messages" : `${feed.length} messages`
```

A source streams `feed`, answers its first message with a continuation (what an
`async` mapper that awaits `scheduler.yield()` does), and gives two items in its next
turn.

```
open, no message yet:                   direct "connecting"; derived "connecting"; "No messages"; outstanding clear false
first message arrived, mapper awaiting: direct "live";       derived "connecting"; "No messages"; outstanding clear true
the mapper's value landed:              direct "live";       derived "live";       "2 messages";  outstanding clear true
```

Expected: the middle line reads "connecting" twice, and `outstanding` lists the stream.

## Verification

- The runner: a temporary test in `runner/tests/it/` compiled the app above, ran it
  over a Rust `DataSource` and printed the three lines.
- The web JS target: the same view without the derive, over a TypeScript source whose
  `exactStream` mapper is `async e => { await gate; return ["a", "b"]; }`, built with
  `bun host/web-js/build.mjs` and driven in headless Chrome through
  `scripts/agent.mjs`. With the first event delivered and the mapper awaiting, the
  view read "connecting" and `state.pending` listed `feed`; once `gate` resolved it
  read "live", "2 messages".

## Direction

Count a stream's first message as landed when its mapper's value commits: while a
mapper turn is out for a stream that has committed no value, keep it in flight.

## Related

`issues/closed/20261023-a-stream-mapper-cannot-yield.md` added mapper turns. Its
tests open each stream with a message mapped at once.
