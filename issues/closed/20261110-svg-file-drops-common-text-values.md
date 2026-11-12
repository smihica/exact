# Text in an SVG image draws `font-weight="bold"` at regular weight and ignores every `font-family`

**Status:** Closed
**Resolution:** SVG images resolve their own font stacks and convert weight keywords, absolute font sizes, oblique style, and percentage opacities for native drawing.
**Systems:** host/svg-file/src/lower.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The file reader hands each presentation attribute to the kernel's style row as text
and, when the row refuses it, tries it once more as a bare number
(`host/svg-file/src/lower.rs:383-404`). The rows take Contract's values, not SVG's.
A value SVG defines and the row does not take is dropped with the warning "is not a
value the row takes", and the element draws with the inherited or initial value.

LLP 1011.001 makes a file work "the way a browser's `<img src="x.svg">` works" and
declares two deviations (animation, and the natural size of a `viewBox` alone). The
values below are neither. They are of two kinds.

**Values the reader does not convert.**

| In the file | The scene has | Chrome computes |
|---|---|---|
| `font-weight="bold"` (`"bolder"` is refused the same way) | weight 400 | 700 |
| `font-weight="normal"` on a `tspan` inside weight 700 | weight 700 | 400 |
| `font-size="9pt"` | 16 px | 12 px |
| `font-style="oblique"` | upright | oblique |
| `opacity`, `fill-opacity`, `stroke-opacity` of `50%` | the attribute dropped | 0.5 |

**`font-family`, which no value can set.** The row is an index into a plan's font
stacks (`u16`, `kernel/tables/schema.json:3323-3326`). The compiler writes it for an
inline `svg`, and the runner interns a family named at run time
(`issues/closed/20261009-runtime-installed-font-family.md`). A file's scratch kernel
has neither, so every family a file names is refused and its text draws in the
default family. LLP 1011.001 D2 lists "`text` with the host's fonts" among what a
file draws as the inline `svg` does.

## The failure

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
  <text style="font-weight:bold;font-family:Arial,Helvetica,sans-serif;font-size:9pt">x</text>
</svg>
```

Observed, `exact_svg_file::load(file).document.warnings`:

```
`font-weight="bold"` on `<text>` is not a value the row takes
`font-family="Arial,Helvetica,sans-serif"` on `<text>` is not a value the row takes
`font-size="9pt"` on `<text>` is not a value the row takes
```

Expected: bold, 12 px, in Arial where the host has it
([CSS Fonts 4 §2.2](https://www.w3.org/TR/css-fonts-4/#font-weight-prop): `bold` is
700).

## Verification

Run: a temporary test in `host/svg-file/tests` loaded one element per attribute and
printed the warnings and the run's font in `load(file).scene(200, 40)`. For
`font-weight="bold"` the run's style was `font_size: 16.0, font_weight: 400,
font_style: Normal`; for `"700"` it was `font_weight: 700`. Accepted in the same run:
`font-size="12"`, `12px` and `1em`, `font-style="italic"`, `opacity=".5"`. The middle
column is from that run. The right one is `getComputedStyle` on the same elements in
headless Chrome 156, except the `normal` row, which is CSS's definition. How a host
then draws the scene was not run.

## Direction

Convert in the reader before the row sees the value: the weight keywords to numbers,
absolute units to px (the reader does this for the root's `width`, `lib.rs:105-130`),
a percentage opacity to its fraction, `oblique` to italic. `font-family` needs a
decision first: the scene's run would have to carry the family names for the host to
resolve, since a file has no plan to index.

## Related

`QUEUE.md`, "CSS font generic mappings…", is about generics in app styles. LLP
1055.000 D4 keeps `stroke-width` a number, so `stroke-width="1pt"`, also refused, is
not part of this.
