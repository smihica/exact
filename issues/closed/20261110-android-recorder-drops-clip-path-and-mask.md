# On Android `clip-path`, `mask-image` and an SVG `clipPath` clip nothing

**Status:** Closed
**Resolution:** Android records CSS and SVG path clips and composites gradient masks over isolated content.
**Systems:** host/linux/src/canvas.rs (`canvas::Recorder`), host/linux/src/paint/backend.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. The paint walk asks its backend for three kinds of clip through trait methods that have default bodies (`host/linux/src/paint/backend.rs:255`, `:262`, `:271`, `:276`):

```rust
fn push_svg_clip(&mut self, _clip: &…::Clip, _ts: Transform) -> usize { 0 }
fn push_css_clip(&mut self, _path: &…::ClipPath, _ts: Transform) -> bool { false }
fn push_mask(&mut self, _shape: &Shape, _ts: Transform) { self.push_opacity(1.0); }
fn pop_mask(&mut self, _shape: &Shape, _mask: &…, _ts: Transform) { self.pop_opacity(); }
```

The CPU, GPU and Ink backends implement all four (`raster.rs:1075` to `:1191`, `gpu.rs:933` to `:1006`, `ink.rs:605` to `:648`). The Canvas recorder, which is Android's painter, implements none (`impl Backend for Recorder`, `host/linux/src/canvas.rs:509` to `:925`). The walk takes "no clip was pushed" as an answer and paints the content whole (`paint.rs:1071`, `:1077`, `paint/region.rs:783`, `paint/svg.rs:569`).

**The failure.**

```
view width=100 height=100 background-color="#ff0000" clip-path="path('M 0 0 L 100 0 L 0 100 Z')"
view width=100 height=100 background-color="#ff0000" mask-image="linear-gradient(#000000, transparent)"
svg viewBox="0 0 100 100" width=100 height=100
  defs
    clipPath id="c"
      circle cx=50 cy=50 r=30
  rect width=100 height=100 fill="#ff0000" clip-path="url(#c)"
```

- observed on Android: three whole red squares;
- expected (CSS Masking 1 §5.1 `clip-path`, §6.1 `mask-image`, and its `clipPath` element), and what Linux and Windows draw: a triangle, a square fading to nothing, a disc.

`mask-image` is admitted "held to Chrome" (`rules/DEFERRED.md`, LLP 1077 D2), and neither LLP 1077 nor LLP 1055.000 declares an Android gap.

**Verification.** Run: a temporary test in `host/linux/src/canvas/stream.rs` booted a page with a red box holding a blue one, and an `svg` with a red `rect`, through `PainterChoice::Canvas`, and listed each finished stream's ops by code (fonts left out):

```
css plain      [1, 2, 2]
css clip-path  [1, 2, 2]
css mask-image [7, 1, 2, 2, 6]        (a LAYER at alpha 1 and its RESTORE)
css overflow   [1, 2, 4, 2, 6]        (overflow="hidden", a clip the recorder does write)
svg plain      [1, 3]
svg clipPath   [1, 3]
```

Not run: the reader on a device; a stream with no clip op gives it nothing to clip by.

**Every default the recorder inherits.** `Backend` has twenty methods with default bodies that the recorder does not override, leaving out those compiled only for Windows or macOS. Five change what is drawn:

| Method | Default | What Android does not draw |
|---|---|---|
| `island_image` | draws nothing | text shadow, text stroke, `background-clip: text`, an SVG filter or blend mode (filed, below) |
| `push_svg_clip` | 0: no clip | an SVG `clipPath` |
| `push_css_clip` | false: no clip | `clip-path` |
| `push_mask`, `pop_mask` | an opacity layer at 1 | `mask-image` |

The other fifteen manage frames and devices, and their defaults suit a recorder: `page` (the reader paints the page's colour, as the trait says), `needs_geometry_paint`, `lost_device_needs_cpu`, `trim`, `recycle_frame`, `damage`, `begin_damage`, `split_window_canvas`, `recycle_window_layers`, `take_window_underlay`, `window_overlay_rows`, `finish_for`, `row_local`, `row_reusable` and `last_frame_ms`.

**Direction.** Write the two path clips as `CLIP_PATH` (op 5), which the reader already plays, and return the count pushed. A mask needs the group's layer drawn through the gradient: a `SHADER` before the layer's `RESTORE` with a destination-in paint in `Player`, or the mask rasterized as the island pictures are. Removing the five default bodies makes a backend without them fail to compile.

**Related.** `issues/20261110-android-recorder-drops-island-images.md` is the same mechanism for `island_image`; this is the other four methods. `issues/20261110-android-recorder-ignores-paint-order-and-miterlimit.md` is an argument the recorder drops, as its `image` drops the tint.
