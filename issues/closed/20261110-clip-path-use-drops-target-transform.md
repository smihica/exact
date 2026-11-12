# A `use` inside a `clipPath` drops its target's own transform

**Status:** Closed
**Resolution:** A use inside a clipPath applies its target's own transform before the use's placement.
**Systems:** kernel/src/svg/scene/clip.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Resolver::clip` (`kernel/src/svg/scene/clip.rs:92-129`) turns each child of a `clipPath`
into a path in the clipped element's user space. For a `use` child it takes the target
shape's geometry and maps it by the `clipPath`'s matrix, the `use`'s own transform and
the `use`'s `x`/`y` (`clip.rs:122-126`: `base · m · at`, where `m` is
`self.transform(&c, …)` for the `use`). The target's own `transform`, `translate`,
`rotate` and `scale` are never read.

A shape written directly in the `clipPath` keeps its transform, because there `c` is the
shape. A `use` anywhere else instances its target through `Resolver::item`, which applies
the target's transform (`kernel/src/svg/scene.rs:445-449`). A `use` renders its target
with the target's own transform (SVG 2 §5.5), and CSS Masking 1 §6.1 makes no exception
for a `use` in a `clipPath`.

## The failure

```
svg width=100 height=100 viewBox="0 0 100 100"
  defs
    rect id="s" width=10 height=10 transform="translate(50 0)"
  clipPath id="c"
    use href="#s"
  rect width=100 height=100 fill="red" clip-path="url(#c)"
```

Chrome 156 shows the red square at `50,0`, as it does with the `rect` written directly in
the `clipPath`. The macOS app shows it at `0,0`.

## Verification

- A kernel test resolving both scenes printed the clip shape's bounds:
  `(50.0, 0.0, 10.0, 10.0)` with the `rect` as the `clipPath`'s child and
  `(0.0, 0.0, 10.0, 10.0)` with the `use` of it.
- The snippet and its control (the `rect` in the `clipPath`), put in `apps/hello`
  temporarily, built with `bun host/apple/build.mjs hello-apple` and captured with
  `bun scripts/agent.mjs macos --app hello "screenshot out.png"`: the control paints 100
  red pixels at x 50 to 59, y 0 to 9 of its `svg`; the `use` paints them at x 0 to 9.
- Headless Chrome 156, the same two documents drawn to a canvas: the pixel at `5,5` is
  transparent and the pixel at `55,5` is `rgba(255, 0, 0, 255)` in both.
- The Linux host was not run. It draws the clip from the same `Clip::shapes`.

## Direction

Multiply the target's own transform in after the `use`'s placement:
`base · m · at · self.transform(&shape, &ss, vp)`.

## Related

No issue names a clip's `use`, and the `QUEUE.md` entry "LLP 1055 / 1055.000 (SVG,
completed in stages) owes" does not list it.
