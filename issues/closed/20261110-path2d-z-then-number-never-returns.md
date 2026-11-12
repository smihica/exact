# `new Path2D(d)` never returns when a number follows `Z`

**Status:** Closed
**Resolution:** A number after `Z` ends the path data in both recorders, as an error does, instead of repeating `Z` for ever.
**Systems:** canvas/src/path2d.rs, canvas/recorder.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Both recorders parse SVG path data in a loop that repeats the last command when the
next token is a number (`canvas/src/path2d.rs:559-567`, `canvas/recorder.js:699-702`).
Every command but one consumes its numbers. `Z` consumes nothing
(`path2d.rs:647-652`, `recorder.js:724`), so when a number follows `Z` the loop
repeats `Z` at the same input position for ever, and each turn appends one more close
segment to the path.

A close-path takes no parameters, so a number after it is an error, and the module's
own rule is that parsing "stops at the first error and keeps what came before it, as
SVG and Chrome do" (`path2d.rs:13-14`).

## The failure

```rust
let p = exact_canvas::Path2d::new_with_path_string("M0 0 L10 0 Z 5");
```

```js
new Path2D("M0 0 L10 0 Z 5")   // in a data module's draw
```

Observed: the call does not return, and the process grows until it is killed. The
draw that made the call never finishes.

Expected: the path `M0 0 L10 0 Z`, as Chrome builds it
([SVG 2 §9.3.4](https://www.w3.org/TR/SVG2/paths.html#PathDataClosePathCommand): `Z`
takes no parameters;
[§9.5.4](https://www.w3.org/TR/SVG2/paths.html#PathDataErrorHandling): the path is
rendered "up to (but not including) the path command containing the first error").

## Verification

Run, all three:

- Rust: a temporary test in `canvas/tests` parsed the string on a thread and waited
  20 ms. It had not returned, and the process's resident size went from 51 MiB to
  146 MiB in the next 40 ms.
- TypeScript: `canvas/recorder.js` under Bun, each path in a child process with a
  300 ms limit. `"M0 0 L10 0 Z"` returned; `"M0 0 L10 0 Z 5"` was killed at the limit.
- Headless Chrome 156: `new Path2D('M10 10 L60 10 L60 60 Z 5')`, and the same with
  `z1 2`, returned in under 1 ms and `fill(path)` painted the triangle.

## Direction

A number where a command is expected repeats the last command only when that command
takes parameters: after `Z`, stop. The same change in both recorders, with the case
added to `canvas/tests/cases.txt` so both stay held to it.

## Related

`issues/closed/20260927-clip-path-path-hangs.md` is this defect in the kernel's `d`
parser (`kernel/src/svg/path.rs`), fixed there by refusing a number after `Z` and
asserting that each command consumes input. The canvas crate keeps a parser of its own
in f64 (`path2d.rs:14-15`) and the TypeScript recorder a third; neither took the fix.
Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names `Path2D` parsing.
