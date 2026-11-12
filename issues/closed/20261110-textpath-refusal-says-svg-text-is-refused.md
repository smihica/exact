# The refusal of `textPath` says text inside `svg` is refused, which it no longer is

**Status:** Closed
**Resolution:** The textPath refusal names stage 11 and recommends positioned text with tspan runs inside svg.
**Systems:** contract/lower/src/svg.rs (`refused_tag`), contract/lower/src/lint.rs (`unknown_tag`)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`svg::refused_tag` (`contract/lower/src/svg.rs:336-352`) names the SVG tags outside the subset for `lint::unknown_tag`. Its first arm is

```rust
"text" | "tspan" | "textPath" => {
    "text inside `svg` is refused (LLP 1055 D12); put a `text` beside the `svg`"
}
```

SVG text is built (LLP 1055.000 D11, stage 5): `tags::tag` has `tspan` (`contract/lower/src/tags.rs:271`), `svg::holds` lets an `svg` or `g` hold `text` and a `text` hold `tspan` (`svg.rs:108`, `svg.rs:124`), and `svg::coerce` lowers their `x`, `y`, `dx` and `dy` (`svg.rs:1228-1247`). Only `textPath` still reaches this arm, and the sentence it prints is false: it tells the author to move text out of the `svg`, when a `text` with `x` and `y` inside it is what stands in for a `textPath`.

The `"text"` and `"tspan"` patterns of that arm, and `"image"` of the next, are dead: all three are known tags, so `unknown_tag` is never called with them.

## The failure

```
component A
  view
    svg width=100 height=40
      textPath "hi"
```

Observed:

```
4:7 [lower-unknown-tag] unknown tag `textPath`; text inside `svg` is refused (LLP 1055 D12); put a `text` beside the `svg`
```

`contract vocab textPath` prints the same sentence.

Expected: a refusal that says `textPath` (text on a path) is a later stage (LLP 1055.000 §4, stage 11), as `feImage` is told, and that `text` and `tspan` are what an `svg` takes.

The same file with `text "hi" x=10 y=20` in place of the `textPath` compiles, as does `text x=10 y=20` holding `tspan "hi"`.

## Verification

At `dd6f5dcd`, a temporary test in `contract/cli/tests` compiled the three sources: the two with `text` compiled, and the `textPath` one was refused with the line above. `contract vocab textPath` printed "`textPath` is not a tag, an attribute, a standard function, a recipe or a pattern; text inside `svg` is refused (LLP 1055 D12); put a `text` beside the `svg`".

## Direction

Reword the arm for `textPath` alone and delete the `text`, `tspan` and `image` patterns that no caller reaches.

## Related

`QUEUE.md`'s "LLP 1055 / 1055.000 (SVG, completed in stages) owes" lists `textPath` as unbuilt; it does not mention the message. No issue or `docs/agent-pitfalls.md` entry names `textPath` or `refused_tag`.
