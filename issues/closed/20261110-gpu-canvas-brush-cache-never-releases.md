# The GPU painter keeps every canvas snapshot it has drawn for the life of the process

**Status:** Closed
**Resolution:** The GPU canvas brush cache evicts snapshots once only the entry and brush own them.
**Systems:** host/linux/src/gpu.rs, host/linux/src/canvas2d
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Gpu::canvas` (`host/linux/src/gpu.rs:763-803`) caches one image brush per snapshot,
keyed by the snapshot's address, and means to drop an entry once no painter holds the
snapshot:

```rust
// A snapshot no painter holds any more is dropped with its brush.
self.canvas_brushes
    .retain(|_, (pixels, _)| Arc::strong_count(pixels) > 1);
```

The entry itself holds two references: the `Arc<Pixmap>` stored beside the brush
(line 788) and the one inside the brush's blob (line 782). The count never falls
below 2, so `retain` removes nothing, and nothing else clears the map.

Each revision of a 2D canvas is a new `Arc` holding a copy of the whole bitmap
(`canvas2d/mod.rs:563-568`), as is each hatch overlay (`paint.rs:1294-1298`) and,
under `EXACT_GPU_RENDER=1`, each GPU canvas readback (`surfaces/pixels.rs:72-78`).
Every one drawn stays allocated until exit: width × height × 4 bytes at device scale.

This is the Linux host's default painter: no app bakes one, and `PainterChoice::Auto`
takes it wherever there is a GPU adapter (`presenter/painter.rs:229`). The agent and
the smokes launch with `EXACT_PAINTER=cpu` (`scripts/agent-stdio.mjs:80`) and never
reach it.

## The failure

Caltrain's line map is a 2D canvas whose arguments include `nowMs`, which
`every(1000, tick)` changes (`apps/caltrain/app.contract:190`, `374`), so it is redrawn
once a second. At 1280 × 800 its box is 600 × 240 points: 576,000 bytes at scale 1.

`caltrain-linux` (`--profile host-dev`, headless on a Mac) with its clock advanced one
second at a time for 60 seconds and a frame painted after each:

| Painter | Resident size at start | After 60 s |
| --- | --- | --- |
| `EXACT_PAINTER=gpu` (Metal) | 68,336 KB | 111,472 KB |
| `EXACT_PAINTER=cpu` | 39,312 KB | 44,464 KB, flat after the first 10 s |

Expected: at most the snapshot in use. Observed: about 0.6 MB a second, 2 GB an hour,
and more at a higher scale.

## Verification

Run on Metal. The table is `ps -o rss=` on the process while a `scripts/agent.mjs`
session ran `clock +1000` and `screenshot` 60 times.

A temporary test in `gpu.rs` drew 100 frames, each calling `gpu.canvas` with a fresh
800 × 600 `Arc<Pixmap>` that the caller then dropped. It left 100 entries, each with a
strong count of 2, all 100 snapshots alive, and 192,000,000 bytes of pixels held. A
second test booted a presenter on a plan with a 400 × 300 `canvas` at scale 2 and
redrew it 60 times: under `PainterChoice::Cpu` one of the 60 snapshots was alive at
the end, under `PainterChoice::Gpu` all 60.

Not run: Vulkan, a KMS display.

## Direction

Compare against the entry's own two references (`> 2`), or keep only the brush in the
map and test its blob's pixmap, so an entry goes when the caller's last reference does.

## Related

`issues/20261110-vello-clip-layers-grow-every-frame.md` is the canvas crate's replayer,
a different cache. `issues/closed/20261006-worker-retirement-queue-unbounded.md` is the
Apple owner queue. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names
`canvas_brushes`.
