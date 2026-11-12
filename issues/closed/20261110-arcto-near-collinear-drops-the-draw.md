# `arcTo` at a corner that doubles back almost exactly records NaN, and the whole draw is dropped

**Status:** Closed
**Resolution:** Both arcTo recorders use Chrome's 1/4096 collinearity tolerance, keeping near-reversal draws finite and drawing a line to the corner.
**Systems:** canvas/src/geom.rs, canvas/recorder.js, runner/src/runner/canvas2d.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`geom::arc_to` takes the three points as collinear only when the sine of the angle at
the corner is at most 1e-12 (`canvas/src/geom.rs:208`; the same test at
`canvas/recorder.js:423`). Above that it puts the tangent point `r / tan(half)` from
the corner (`geom.rs:213-214`, `recorder.js:426`). When the path doubles back (`p2`
on the same side of `p1` as `p0`) the half angle is tiny:

- Up to about 1.5e-8 rad the cosine rounds to 1, the half angle is 0 and the distance
  is infinite. Both recorders write `LineTo NaN NaN`. `list::check` refuses a list with
  a non-finite operand (`canvas/src/list.rs:445-450`), and the runner then discards the
  whole draw, records the error and starts a new generation
  (`runner/src/runner/canvas2d.rs:779-799`).
- From there up to a sine of 1/4096 the list is finite, and it holds a line to a
  tangent point far off the canvas.

The second is what the HTML standard's arithmetic gives. Chrome does neither: Skia's
`SkPath::arcTo` adds a line to `(x1, y1)` when the sine of the turn is at most 1/4096,
and the recorder is held to Chrome (LLP 1056 §4).

## The failure

```rust
let ctx = Context2d::new();
ctx.begin_path();
ctx.move_to(0.0, 50.0);
ctx.arc_to(100.0, 50.0, 0.0, 50.0 + 1e-7, 5.0)?;   // 1e-9 rad; then + 1e-4 and + 1e-2
ctx.stroke();
```

Observed, the record after `MoveTo 0 50` and what `list::check` returns:

```
1e-9 rad: LineTo NaN NaN            Err(Malformed("LineTo with a non-finite operand"))
1e-6 rad: LineTo -9999455.527 50    Ok
1e-4 rad: LineTo -99900 50          Ok
```

Expected, as Chrome draws all three: `LineTo 100 50`.

## Verification

Run:

- A temporary test in `canvas/tests` printed the lines above from
  `list::describe` and `list::check`. The TypeScript recorder under Bun
  (`new Recorder()`, the same calls) wrote the same three `LineTo` records.
- Headless Chrome 156, the same calls on a 200 by 100 canvas moved to x = 20 and
  stroked 3 px wide, painted exactly x = 20 to 119 for a sine of 1e-9, 1e-6, 1e-4 and
  2.44e-4. At 2.45e-4 and above it runs the line off the canvas, so the threshold is
  the only difference from Chrome.

The runner's handling of a refused list was read, not run.

## Direction

Use Skia's rule in `arc_to` in both recorders: a turn whose sine is at most 1/4096 is
a line to `p1`. Add the case to `canvas/tests/cases.txt`. That also removes the only
path found from finite arguments to a non-finite operand.

## Related

No issue or QUEUE entry names `arcTo`. LLP 1056 §4 lists "`arcTo` cases" as a fixture;
`cases.txt` holds a negative radius, one corner and one exactly straight line.
