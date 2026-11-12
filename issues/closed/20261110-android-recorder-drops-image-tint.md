# On Android an image's `-exact-tint-color` is ignored: a tinted raster draws in its own colours

**Status:** Closed
**Resolution:** Android records image tint and applies it through the image alpha on both image draw paths.
**Systems:** host/linux/src/canvas.rs, host/android/canvas/src/main/java/dev/exact/canvas/Player.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. `-exact-tint-color` on a raster `image` draws the picture's shape in that colour. The walk hands every backend the tint (`host/linux/src/paint.rs:86`, `image_tint`, passed at `paint.rs:1168`, `paint/rows.rs:423` and `paint/region.rs:292`), and the CPU painter (`host/linux/src/raster.rs:867` to `:900`) and the GPU painter (`host/linux/src/gpu.rs:737` to `:754`) fill it over the picture through the picture's alpha. The guide states the rule: "`-exact-tint-color` colours a raster image" (`docs/contract-for-agents.md:4185`, `docs/agent-pitfalls.md:146`).

The Canvas recorder, which is Android's painter, takes the argument and drops it (`host/linux/src/canvas.rs:610`):

```rust
fn image(&mut self, image: &Arc<Bitmap>, dst: Rect4, clips: &[Shape], ts: Transform,
         _tint: Option<[u8; 4]>) {
    …
    self.picture(&Picture::Bitmap(image.clone()), dst, clips, ts);
}
```

The stream has no word for a tint: `IMAGE` (op 8) and `IMAGE_RRECT` (op 14) carry an id and rectangles, and `Player.kt:770` and `:801` draw the bitmap with `imagePaint`, which has no colour filter.

**The failure.**

```
image "icons/star.png" width=24 height=24 -exact-tint-color="red"
```

- observed on Android: the PNG in its own pixels (a black template icon stays black, in dark mode too);
- expected, and what the Linux and Windows painters draw: the icon's shape in red.

**Verification.** Run: a temporary test in `host/linux/src/canvas/slot.rs` recorded one 10 by 10 bitmap through `Recorder::image` with `None` and with `Some([255, 0, 0, 255])`, and asserted that the two streams differ. The assertion fails; both are

```
[11, 1, 10, 10, 1, 1065353216, 0, 0, 1065353216, 0, 0, 14, 1, 0, 0, 1092616192, …]
```

(an `IMAGE_DEF`, a `MATRIX`, an `IMAGE_RRECT`). Not run: the reader, for which no device was available; its two image ops were read.

**Direction.** Carry the tint in the stream (a word on ops 8 and 14, or an op before them as `SHADER` precedes a shape) and have `Player` draw the bitmap through a `PorterDuffColorFilter(tint, SRC_IN)`. The ops' lengths are repeated in `ExactView.ingest`, `Structure.end`, `Bounds.measure`, `Slots.kt` and `stream.rs`'s `end`.

**Related.** `issues/closed/20260927-web-tint-masks-the-box.md` is the web's mask. `QUEUE.md`'s tint entries are about symbols and native buttons. `issues/20261110-android-recorder-ignores-paint-order-and-miterlimit.md` is another argument the recorder drops, for SVG strokes.
