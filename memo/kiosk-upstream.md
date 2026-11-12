# Kiosk changes and upstream assessment

Snapshot: 2026-11-12, `kiosk` at
[`95085e29`](https://github.com/smihica/exact/commit/95085e29f6350003d25f4d26185aeeccc30a856d).
This fork evaluates Exact on a Raspberry Pi 4 touchscreen kiosk, using Ink and
DRM/KMS without a desktop environment. The aim is responsive UI, camera preview
and touch scrolling on a small GPU. The changes are experiments with measured
benefits and remaining integration work.

Read [KIOSK.md](../KIOSK.md) for runtime switches, prototype limits and checks.
This memo maps the changes to their code and assesses what is worth sharing
upstream. The assessments are the fork's, not acceptance decisions by Exact's maintainers.

## Changes and value

| Change | Current implementation | Value to upstream | Work before general adoption |
| --- | --- | --- | --- |
| [Shader specialization](../host/ink/src/device/wgpu.rs) | Compile a wgpu pipeline per primitive, brush and flag class to reduce register pressure. | High: the V3D measurements show a large benefit. | Bound the program count and first-use cost; measure other GPUs and draw-call overhead. A small fixed set built with the device, or a V3D-specific approach, is worth evaluating. The current implementation affects every host selecting Ink's wgpu device. |
| [Direct output](../host/ink/src/device/wgpu/scanout.rs) | Present through a Vulkan DRM surface instead of reading every frame back to the CPU. | Very high: this follows the [Linux host's stated direction](../host/linux/src/display.rs). | Select it when available with an appropriate fallback; preserve pixel capture/VNC semantics and investigate the three presents currently used when attaching the display. The current [finish path](../host/linux/src/ink.rs) presents even for a pixel-only request on an attached painter. |
| [GPU rotation](../host/ink/src/device/wgpu/scanout.rs) | Rotate the final image for a portrait panel; [input mapping](../host/linux/src/display/kiosk.rs) follows the orientation. | High for sideways-mounted kiosk panels. | Define a host orientation setting and consistent display/input coordinates. The current probe's fixed turn is not a complete orientation API. |
| [GPU limits](../host/ink/src/device/wgpu.rs) | Request the adapter's supported limits in Ink and [Linux Vello](../host/linux/src/gpu.rs). | Very high: without the adjustment, device creation fails on Pi 4. | Use the existing [`requested_limits`](../gpu/src/device.rs) approach to choose what the renderer needs, rather than requesting every adapter maximum. |
| [Split render/display devices](../vendor/wgpu-hal/src/vulkan/drm.rs) | Match the Pi's display connector to a different rendering device. | High beyond the Pi; also relevant to gfx-rs/wgpu. | Keep the dependency fix separate, verify device selection on other configurations and preserve the [vendor patch record](../vendor/wgpu-hal/EXACT-PATCHES.md). |
| [Shadow cache](../host/linux/src/ink/shadows.rs) | Raster eligible fixed geometry once, then reuse alpha with live position, color and opacity. | High: useful for repeated cards and moving dialogs. | Evaluate cache lifetime, changing geometry, allocation failure and first-use work. The current cache has a 16 MiB non-evicting budget and falls back to analytic drawing for uncached geometry after the budget is exhausted. |
| [Outer shadow bands](../host/linux/src/ink/shadows.rs) | Draw the visible bands around the excluded border box. A separate [shader shortcut](../host/ink/shaders/ink.wgsl) skips zero clip coverage for cached shadows, even when bands are disabled. | Useful: reduces the area drawn for outer shadows. | Fix the extra combined-mask work with nested rounded clips or fractional rectangular clips, then measure broader scenes. |
| [Camera preview](../host/linux/src/image/kiosk_camera.rs) | Read and convert each fixed-size frame from `rpicam-vid`; publish only the latest through the Linux image path. Bounded luminance subscribers let apps analyze that same capture. | Useful as a working Pi reference and evidence for Linux camera support. | Fit the capture/preview lifecycle into `@exact/media-capture` and `@exact/camera`, including grants and agent substitutes. Linux camera support remains deferred in the [pinned upstream rules](../rules/DEFERRED.md); widening that scope needs a maintainer decision. |
| [Touch drag and inertia](../host/linux/src/display/kiosk_scroll.rs) | Single-finger vertical drag, fling, cancellation of a pending tap and catching an active fling. | High for a usable Linux touchscreen host. | Connect to the existing reader drag lifecycle and completion/events; define nested-scroller behavior and device classification. |
| [Rubber-band overscroll](../host/linux/src/display/kiosk_scroll.rs) | Resist movement beyond the boundary and spring back; [painting](../host/linux/src/content_region/presenter.rs) keeps visual displacement separate from logical scroll. | Useful alongside touch scrolling. | Fit the platform's behavior and motion preferences. The probe's physics were chosen for comparison with egui. |
| [Touch cursor behavior](../host/linux/src/display/pointer.rs) | Hide the cursor for touch; reveal it on mouse movement. | High, and relatively independent of renderer changes. | Build on the existing [Pointing and touch_only classification](../host/linux/src/presenter/device.rs) to distinguish direct touch from pen/touchpad reports, and handle initial mouse visibility. |
| [Dialog backdrop fade](../host/linux/src/paint/alert.rs) | Under `KIOSK_DIALOG_FADE=1`, replace the scrim with black at `72% × presented dialog opacity`. | Low for this patch as written; useful as a backdrop styling/animation use case. | Put the application's color, opacity and transition choices at the authoring boundary. The fixed black 72% value exists to match the comparison app. |
| Virtual-list comparison | An external app combines images, SVG, Japanese text, rounded corners, shadows and append batches using Exact's existing virtual list. | High as a benchmark or sample, not as a new list implementation. | Publish a small self-contained fixture with matching input and capture procedures. The list and virtualization primitives already belong to upstream. |
| [FPS overlay](../host/linux/src/paint/kiosk.rs) and [timing](../host/linux/src/display.rs) | Show submitted-frame rate and log timing for local comparisons. | Useful measurements; low value for the dedicated overlay as written. | Use existing performance tooling where possible. Actual presented FPS comes from DRM page-flip completion events, not this counter. |

The app owns dialog translation, opacity, duration and easing. The backdrop
override above is the host-side exception, not the implementation of the app's
whole animation. Display rotation uses the GPU on the direct path,
while the camera worker currently converts YUV420 to RGBA and rotates on the CPU.
Camera capture is fixed at 640×480/30fps and produces a 480×640 portrait preview.

## Upstream discussion

[Issue #145](https://github.com/expo/exact/issues/145) and its
[reduced dialog reproduction](https://gist.github.com/smihica/43fc09c307fa2968cee057483c86c99f)
contain the original performance report and patches. That reproduction predates
the camera, touch-scroll and shadow-cache experiments.

- The [first maintainer response](https://github.com/expo/exact/issues/145#issuecomment-6797730474)
  distinguishes the useful shader diagnosis from the cost of open-ended,
  first-use pipelines. It also names direct presentation as the planned Linux
  direction, asks for capture/fallback cleanup, and points to the existing
  GPU-limit selection code. The table's proposed changes address that feedback;
  they are not an agreed implementation plan.
- The [two general early returns were accepted](https://github.com/expo/exact/issues/145#issuecomment-6799709624)
  in [`11043997`](https://github.com/expo/exact/commit/110439978ef35965e79e5842986413fed19a79a7): full coverage deep inside a shape, and skipping the shadow
  integral under a fully covered border box. Upstream rewrote the guards and
  added boundary tests. The fork's merge uses those upstream guards. These are
  already integrated, rather than pending local proposals.
- The fork's [Pi retest](https://github.com/expo/exact/issues/145#issuecomment-6814656945)
  confirms the device-creation failure without the limits adjustment and the
  remaining register spills after the early returns. Specialization is still
  a separate local experiment.

[CONTRIBUTING.md](../CONTRIBUTING.md) currently directs contributions through
issues and reproductions. Fork code changes go through PRs to
`smihica/exact:kiosk`; do not open PRs to `expo/exact`.

## Evidence and its limits

The device is a Pi 4 with 8 GB RAM, Raspberry Pi OS Lite 64-bit / Debian Trixie,
Mesa 26.2.6 / V3D 4.2.14.0, and a 1280×800 panel used as an 800×1280 portrait UI.
Measurements below use release builds. The dialog and list are separate fixtures.

| Comparison | Before | After | Measurement |
| --- | ---: | ---: | --- |
| Original dialog: shader changes plus direct display/GPU rotation | About 6 fps | About 51 fps | Page flips during opening; backdrop opacity held constant throughout the animation. |
| Reduced dialog: specialization off/on, local early returns held constant | 97.34 ms | 17.70 ms | Vulkan GPU time 200 ms into opening; median of 10 warm samples. |
| List: analytic shadows to cached outer bands plus clip shortcut, before the main merge | 49.80 ms | 15.10 ms | Headless GPU rendering at fixed list positions. |
| Same list optimization sequence | 18.75 fps | 54.36 fps | Presented frames in fixed 1.6 s swipe windows. |
| Pre-merge list to main integration | 15.10 ms | 16.45 ms | Fixed-position headless GPU comparison. |
| Pre-merge list to main integration | 54.36 fps | 42.33 fps | Four 1.6 s swipes per version; intervals under 20 ms fell from 313 of 344 to 171 of 267. |

The last comparison ran at 35–36°C without throttling. The regression's cause
is unresolved: the merge includes changes beyond shader early returns, so the
measurements do not attribute it to those guards. Shader first-use cost, sustained
operation and the other-GPU tradeoff remain separate questions from warm throughput.

The original shader comparison matched 29 PNGs per backend byte-for-byte.
Pre-merge shadow changes stayed within 2/255 of analytic rendering in the sampled
captures. After the main merge, integer list positions matched the prior version;
fractional positions differed at photo edges, by up to 27/255. This is consistent
with upstream image clipping changes, but has not been isolated to one commit.
List measurements use an external consuming app; its fixture and traces are
maintained separately from this fork.

At this snapshot, focused native checks passed: 14 kiosk tests, 30 Linux Ink
tests and 34 exact-ink tests, with one existing ignored test. The consuming app's
three Contract tests, ARM64 release build and camera start/stop/restart, failure,
stalled-capture cleanup and dialog checks also passed. These establish the tested
paths, not general camera support or long-running stability.

The full test suite was not green: four Linux presenter/text cases, an Apple
inherited-layout case and a runner incremental case also failed on clean upstream
`bd60756a`. Executor/process deadlines and secrets-helper failures varied between
runs and occurred on the clean baseline too. These results describe the pinned
integration revision; see [KIOSK.md](../KIOSK.md) for check commands.

## Continuing from this snapshot

- Original experiment base: `fe4632ed1800ce58f3bcfa880e95f834395d77d9`.
- Pre-merge feature stack: `f90ba38adc9088a6e412e1b522fa547a20660e7b`.
- Integrated upstream revision: `bd60756a7fcd9f1d3c406886d8ed3075057800d7`.
- Tested integration: `49959ca0f6edd336de83a7039c731aae1f62cc7a`.
- [Fork PR #1](https://github.com/smihica/exact/pull/1) merged as `95085e29`,
  whose tree is identical to the tested integration. The performance regression
  remains a follow-up.

To inspect fork work after upstream moves:

```sh
git fetch upstream main
git diff upstream/main...kiosk
```

The three-dot comparison starts at the common ancestor and excludes upstream
commits not yet integrated into `kiosk`. At this snapshot the fork delta is
34 files, +2,365/-127 lines, before adding this memo.

A useful next report would pair the shadow-cache and outer-band results with a
small public list fixture. One hypothesis for the main-merge regression is the
change to `shape_coverage` guards. Compare the old and upstream guards with the
host and fixture held constant to isolate that effect before drawing a conclusion.
