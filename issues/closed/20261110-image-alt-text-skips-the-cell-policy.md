# An image's alternate text is put in cells without the cluster policy: a control character is written as itself and a wide cluster takes one cell

**Status:** Closed
**Resolution:** An image's alternate text goes through the painter's one policy: a control character is a replacement glyph, whitespace a space and a wide cluster two cells.
**Systems:** host/terminal (paint.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Every other text the painter draws goes through one policy: `measure::cell` for paragraphs and button titles, `paint::clusters` for fields. Both replace a cluster holding a control character with U+FFFD and give a wide cluster two cells (`host/terminal/src/measure.rs:315-331`, `host/terminal/src/paint.rs:865-877`, at `dd6f5dcd`).

The image fallback does neither. An image shows its alternate text when it has no decoded picture (the host decodes local PNG files only, `host/terminal/src/image.rs`) or the palette is monochrome. `image` then writes `[<aria-label>]` one grapheme per cell, each with a width of 1, straight from the prop (`paint.rs:765-772`):

```rust
for (x, g) in (content.x..).zip(format!("[{alt}]").graphemes(true)) {
    out.grid.put(x, content.y, g, 1, style, inner);
}
```

LLP 1101 D7 states the rule: "C0/C1 controls and ESC in app text are drawn as visible replacement glyphs". An `aria-label` is app text, and it is commonly bound to data. A control byte in it reaches the terminal, which acts on it, so the screen no longer matches the grid.

## The failure

```contract
component App
  state label = "a\tb"
  view
    column
      text "first row"
      row width="20ch"
        image "/nonexistent/none.png" width="8ch" height="1lh" aria-label=label
        text "|after"
      text "last row"
```

| `label` | Grid cells | Grid row | Screen row |
|---|---|---|---|
| `"a\tb"` | `[` `a` TAB `b` `]` | `[a<TAB>b]   \|after` | `[a      b]   \|after` |
| `"a\nb"` | `[` `a` LF `b` `]` | `[a<LF>b]   \|after` | `[a` |
| `"x\u{1b}[31mred"` | `[` `x` ESC `[` `3` `1` `m` `r` | `[x<ESC>[31mr\|after` | `[xr\|after` |
| `"x\u{1b}[2Jy"` | `[` `x` ESC `[` `2` `J` `y` `]` | `[x<ESC>[2Jy]\|after` | `  y]\|after`, and `first row` above it is erased |
| `"日本"` | `[` `日` `本` `]` | `\|after` at column 8 | `\|after` at column 10 |

Expected: each control character drawn as U+FFFD and each wide cluster given two cells. A `text` showing the same state does that: its cells are `x` `�` `[` `3` `1` `m`, and `日` and `本` with a continuation cell each.

## Verification

A temporary test compiled each source with `contract::compile_terminal`, booted it full screen at 22 columns, wrote `grid.diff(None)` into a `vt100` parser (the writer's oracle, LLP 1101 §5) and printed the grid's cells and both rows: the table above. No real terminal was run.

## Direction

Lay the alternate text out with `paint::clusters` (or `measure::cell`) and advance `x` by each cluster's columns, as `field` does.

## Related

LLP 1101 §5 has the smoke cover "the hostile-string case"; no test feeds one to an image. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
