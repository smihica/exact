# A `drop-shadow()` on an SVG element ignores an inherited `color`, and an SVG filter's colours ignore the dark appearance

**Status:** Closed
**Resolution:** SVG drop shadows use the cascaded color, and filter colors follow the host appearance and authored color-scheme.
**Systems:** kernel/src/svg/scene/filter.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A `drop-shadow()` that names no colour takes it from the `color` property (Filter
Effects 1 §6.1), which is inherited. `Resolver::filter`
(`kernel/src/svg/scene/filter.rs:371-377`) passes `node.style.text_color.resolve(false)`.
That value is wrong in two ways.

- `node.style` holds only the rows written on the node. `Resolver::item` has the
  cascaded style (`kernel/src/svg/scene.rs:434`) and uses it for the fill and the stroke
  (`scene.rs:768`), but `filter` is handed the node. A shape whose `color` comes from a
  `g`, the `svg` or the page casts a black shadow.
- `resolve(false)` is the light appearance, and `function` is told `dark = false` for a
  colour written in the function too (`filter.rs:175`). `flood-color` and
  `lighting-color` resolve the same way (`filter.rs:756-767`). So a `light-dark()` pair
  in an SVG filter keeps its light value under a dark appearance, and a role colour
  resolves through the same call. A box's filter takes the appearance from its caller
  (`box_filter`, `filter.rs:293`), and a shape's fill keeps the pair for the host
  (`ShapePaint::color`).

## The failure

```
svg width=100 height=100 viewBox="0 0 100 100"
  g color="#ff0000"
    rect x=10 y=10 width=20 height=10 fill="blue" filter="drop-shadow(8px 8px 0px)"
```

Chrome 156 paints the shadow red. The macOS app paints it black. With `color` on the
`rect` both paint it red.

```
rect x=10 y=10 width=20 height=10 fill="light-dark(#00aa00, #ff00ff)" filter="drop-shadow(8px 8px 0px light-dark(#00aa00, #ff00ff))"
```

Under a dark appearance Chrome paints the rectangle and its shadow `#ff00ff`. The macOS
app paints the rectangle `#ff00ff` and the shadow `#00aa00`. The result is the same with
the pair on `color` and no colour in the function, and with the pair on an `feFlood`'s
`flood-color`.

## Verification

- The snippets in a temporary page in `apps/hello` (`bun host/apple/build.mjs hello-apple`),
  captured with `bun scripts/agent.mjs macos --app hello "screenshot light.png"
  "prefer prefers-color-scheme dark" "screenshot dark.png"`. Light: the shadow's 176
  pixels are `#000000` with `color` on the group and `#ff0000` with it on the `rect`.
  Dark: the fill is `#ff00ff`, and both shadows and the flood are `#00aa00`.
- A kernel test over `scene::resolve` printed the `Op::DropShadow` colour:
  `[0.0, 0.0, 0.0, 1.0]` with `color` on the group, `[1.0, 0.0, 0.0, 1.0]` on the `rect`.
- Headless Chrome 156, the documents drawn to a canvas: the shadow pixel is
  `rgba(255, 0, 0, 255)` with `color` on either element. With `color-scheme: dark` on the
  `svg`, the fill, the shadow and the flood are each `rgba(255, 0, 255, 255)`.
- The Linux host was not run. It draws the same resolved primitives.

## Direction

Pass `Resolver::filter` the cascaded style `item` already holds and read `text_color`
from it. Keep the filter's colours as `ColorValue`s for the host to resolve under its
appearance, as `ShapePaint` does, or give `scene::resolve` the appearance.

## Related

`issues/20261110-drop-shadow-refuses-currentcolor.md` is the grammar refusing the
keyword. This is what the default resolves to once the keyword is omitted or accepted.
