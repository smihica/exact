# The KMS display stops after three frames on a driver that reports no vblank sequence

**Status:** Closed
**Resolution:** A page-flip event for the display CRTC completes its pending frame even when every sequence is zero.
**Systems:** host/linux/src/display.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`; kernel references are to Linux v6.12.

`receive_flip` (`host/linux/src/display.rs:252-282`) accepts a page-flip event only
when its sequence moved forward from the last one accepted:

```rust
last_sequence.is_none_or(|last| {
    let distance = e.frame.wrapping_sub(last);
    distance != 0 && distance < 1 << 31
})
```

`e.frame` is the kernel event's `sequence` (`drm` 0.15, `control/mod.rs:1165`). For a
device whose driver never calls `drm_vblank_init`, the kernel stamps every flip event
with sequence 0 (`drivers/gpu/drm/drm_vblank.c`, `drm_crtc_send_vblank_event`):

```c
if (drm_dev_has_vblank(dev)) {
	seq = drm_vblank_count_and_time(dev, pipe, &now);
} else {
	seq = 0;
	now = ktime_get();
}
```

The atomic helpers send the event that way for such a device
(`drm_atomic_helper_fake_vblank`, called from `drm_atomic_helper_commit_tail`). In
v6.12 `simpledrm` (the firmware framebuffer) and `virtio-gpu` (a QEMU guest) are two:
each flips through `drm_atomic_helper_page_flip` and has no vblank code. Which drivers
those are changes between kernels.

On such a card the first flip is accepted (`last_sequence` is `None`) and records
`Some(0)`. Every later event has distance 0 and is refused, so `FlipState::ready`
never takes `pending` (`display.rs:230-245`). The loop paints only when no flip is
pending (`display.rs:424`), so nothing is painted again.

The comparison protects nothing: one flip is outstanding at a time
(`FlipState::submit` refuses a second, `display.rs:216-218`) and the kernel sends one
event per flip.

## The failure

`EXACT_DRM=/dev/dri/card0 caltrain-linux` on a card whose driver has no vblank
counter. Frame 1 is shown by `set_crtc`, frame 2 by a flip whose event is accepted,
frame 3 by a flip whose event is refused. The screen then stays on frame 3: timers,
input and commits continue and no frame follows. Expected: each flip's event releases
the back buffer and the next frame is painted.

## Verification

Read, not run: `display.rs` compiles only on Linux and needs DRM master, and this
review ran on a Mac. The acceptance rule and `FlipState` were copied into a standalone
program and given the events of six frames. With increasing sequences it printed
`6 of 6 frames acknowledged`; with sequence 0 it printed
`frame 4 is never painted: the flip of frame 3 is still pending` and
`2 of 6 frames acknowledged`. The kernel's behaviour is from its source, not
observed. The one recorded hardware run (LLP 1015 §6) was on a Radeon, whose driver
has a vblank counter.

## Direction

Accept the event for the CRTC whenever a flip is pending and drop the sequence
comparison; keep the sequence only as the sample `frames.flipped` reports.
`display/tests.rs` `duplicate_and_older_sequences_cannot_retire_a_successor` and
`duplicate_event_does_not_release_a_new_submission` assert the present rule and
change with it.

## Related

`QUEUE.md`, "Run the Linux host on KMS" asks for a hardware qualification of first
pixel and page flips and does not name this. No issue mentions `receive_flip`.
