# Text in an SVG image loses the `tspan`s beside character data, and the spaces between spans

**Status:** Closed
**Resolution:** SVG images preserve mixed character data and tspans in document order, collapse spaces across runs, and trim only each text chunk's edges.
**Systems:** host/svg-file/src/lower.rs, kernel/src/svg/scene/text.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The file reader gathers a `text` or `tspan` element's own character data into one
string, trims and collapses it with `split_whitespace().join(" ")`, and sets it as the
element's `text` prop (`host/svg-file/src/lower.rs:277-316`). A document mixes
character data and child elements freely, and two things go wrong:

1. **Spans beside text are dropped.** The scene resolver draws a node's `text` prop
   and returns without visiting its children (`kernel/src/svg/scene/text.rs:190-194`).
   An element with both character data and `tspan` children draws only its own data,
   joined across the children, and no warning says so.
2. **Spaces at a span's edge are dropped.** Trimming each element's string removes
   the space that separates it from its neighbour. The resolver already collapses
   white space across runs and trims only a chunk's edges (`text.rs:79-96`,
   `:112-121`), so the reader's trim is what loses it.

The second breaks LLP 1011.001 D2, "Everything the inline `svg` draws (…) draws from
a file": the file form of `apps/svg-gallery`'s `fx-runs` (`tspan "BTC "` then
`tspan "+4.88%"`) draws `BTC+4.88%`.

## The failure

```xml
<text x="0" y="20">Price: <tspan font-weight="700">42</tspan> dollars</text>
<text x="0" y="20"><tspan>Hello <tspan font-weight="700">big</tspan> world</tspan></text>
<text x="6" y="30"><tspan>BTC </tspan><tspan font-weight="700">+4.88%</tspan></text>
<text x="0" y="20"><tspan>A</tspan> <tspan>B</tspan></text>
```

Observed, the run strings of `exact_svg_file::load(file).scene(200, 40)`, with no
warning for any of them:

```
["Price: dollars"]
["Hello world"]
["BTC", "+4.88%"]      (draws BTC+4.88%)
["A", "B"]             (draws AB)
```

Expected, as a browser draws an `<img>` of each
([SVG 2 §11.2](https://www.w3.org/TR/SVG2/text.html#TextElement); white space by CSS
`white-space: normal`): `Price: 42 dollars`, `Hello big world`, `BTC +4.88%`, `A B`.

## Verification

Run: a temporary test in `host/svg-file/tests` printed the four lines above from the
scene's debug form. In headless Chrome 156 `getComputedTextLength()` of the first and
last elements matched that of `Price: 42 dollars` (143.4 px, against 115.6 for
`Price: dollars`) and of `A B` (31.2 px, against 26.7 for `AB`). The nested case was
not measured in Chrome. No host was driven; the scene is what both hosts draw.

## Direction

Keep the document's order: lower each stretch of character data between child
elements as a run of its own (an anonymous `tspan` child carrying the string), so an
element with children has no `text` prop. Pass the string on untrimmed, and let the
resolver collapse and trim as it does for an inline `svg`.

## Related

LLP 1055.000 D11 says a `text`'s "string children and `tspan`s are its content".
`QUEUE.md`, "The compiler's check of a bundled SVG image", is about warnings reaching
the build; this draws wrong with no warning.
