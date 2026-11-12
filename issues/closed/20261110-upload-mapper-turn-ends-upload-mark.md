# An upload whose mapper awaits is recorded with the wrong end: a progress value as its answer, or no answer at all

**Status:** Closed
**Resolution:** Upload marks follow the reply through mapper turns, preserving its final answer or failure without recording progress as the end.
**Systems:** `runner/src/runner/commit.rs`, `runner/src/runner/mapper_turns.rs`, `runner/src/runner/uploads.rs`
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

An upload is asked once for its arguments (LLP 1016.000 D10). Its mark goes from
`Sent(ticket)` to how it ended, and the same arguments, asked again, answer from the
mark without sending. `fulfill_inner` ends the mark when an upload's value lands from
an outcome that is not a stream message (`runner/src/runner/commit.rs:1459-1462`,
`p.upload && !message`). A mapper may now answer with a promise (the amendment of
2026-11-10), and that test is wrong for both of its calls:

- **A progress mapper that awaits.** Its later turns land through `run_turn`
  (`runner/src/runner/mapper_turns.rs:178-203`) with a continuation's outcome, which is
  not a message. `fulfill_inner` knows it is a turn (`commit.rs:1311`), but the upload
  test does not read it, so the first progress value ends the mark while the upload is
  still going out. `Uploads::end` changes only `Sent(ticket)` marks
  (`runner/src/runner/uploads.rs:103-112`): the reply's answer is never recorded, and
  an upload let go after the turn is not marked `interrupted`
  (`runner/src/runner/module_work.rs:119-121`).
- **A reply whose mapper awaits.** The continuation goes out as one more round under a
  new ticket (`commit.rs:1414-1434`), which is no upload (`commit.rs:932`). Its value
  lands and ends nothing: the mark stays `Sent(old ticket)`, which reads as cut off
  (`uploads.rs:299`).

## The failure

```
component App
  state wave = 1
  resource progress = progress(wave) as shape number
  action go(n: number)
    wave = n
  view
    text `${progress}`
```

`progress` answers an upload stream. Its mapper gives the bytes sent for a progress
message and 201 for the reply. After the reply, `go(2)` then `go(1)`:

```
no mapper awaits:          mark Answered(201);  back at wave 1: 0 requests, progress 201
progress mapper awaits:    mark Answered(1024) once its turn lands, and after the reply;
                           back at wave 1: 0 requests, progress 1024
the reply's mapper awaits: mark Sent(1) after its value (201) lands;
                           back at wave 1: 0 requests, progress failed:
                           "the upload was cut off before its reply; it is not sent again at these arguments"
```

Expected in all three: `Answered(201)`, and 201 back at wave 1.

## Verification

A temporary test in `runner/tests/it/` compiled the app above and ran it over a Rust
`DataSource` whose `parse` answers a message, or the reply, with
`Answer::Later(Request::continuation(…))` and gives the value in the next turn. It
read the marks from `Runner::carry().uploads`, and printed the three results above.

Not run through `exact-js`. By reading, `Module::message` (`js/src/lib.rs:1349-1456`)
answers `Answer::Later(Request::continuation(call))` for a message, or an end, whose
mapper call is still pending after its first turn, as the test's source does.

## Direction

Make the test `p.upload && !message && !turn`, and carry `upload` and `sent` onto the
ticket of a reply's next round so its value ends the mark.

## Related

`issues/closed/20261023-a-stream-mapper-cannot-yield.md` added mapper turns; its tests
drive a WebSocket only. `QUEUE.md`, "EX5 upload progress" predates them.
