# Kiosk changes and upstream assessment

The [frame-pacing measurements](frame-pacing.md) cover the fixed Ink programs, solid fills, retained rows, rotated scanout and image downsampling together.

This fork evaluates Exact on a Raspberry Pi 4 touchscreen kiosk, using Ink and
DRM/KMS without a desktop environment. The aim is responsive UI, camera preview
and touch scrolling on a small GPU. The changes are experiments with measured
benefits and remaining integration work.

Read [KIOSK.md](../KIOSK.md) for runtime switches, prototype limits and checks.
This memo maps the changes to their code and assesses what is worth sharing
upstream. The assessments are the fork's, not acceptance decisions by Exact's maintainers.

## Blurred shadows on upstream main (2026-11-27)

The fork merged [`8dd84a33b`](https://github.com/expo/exact/tree/8dd84a33b). It now uses
upstream's one-frame-ahead Vulkan presentation, twelve fixed Ink material programs,
solid interior pass, mip generator, idle photo texture cache, and zoom and border
early exits. The remaining rendering experiment reuses blurred outer shadows. The
[maintainer's integration notes](https://github.com/expo/exact/issues/145#issuecomment-7216650250)
describe the four adopted proposals and the outstanding shadow work.

We built one release app and photo set against three SDKs from that revision.
The first used stock upstream. The second added [shadow texture reuse and exterior
bands](https://gist.github.com/smihica/11871690659079b97dee96eb73236f9b) and
sampled the textures with the general shader. That patch also skips rounded-clip
distance calculations for cached shadows at least four device pixels outside
the exclusion box, where its coverage is zero. The third added [one fixed program
for that shadow material](https://gist.github.com/smihica/882948e965dcb2e293cb2bd36a64872a).
All three binaries embedded the same app plan. On one Pi 4 with Ink/Vulkan, an
800 × 1280 portrait UI and a 60 Hz panel, we replayed a 16-second wheel scroll
through warmed photo rows. Frame rate comes from completed DRM page flips.
GPU time is the V3D render-engine time over the scroll divided by completed frames.

| Shadow rendering | Displayed fps, runs 1 / 2 | GPU ms per displayed frame, runs 1 / 2 |
| --- | ---: | ---: |
| Stock upstream, analytic shadows | 20.59 / 20.53 | 48.38 / 48.50 |
| Shadow patch present, cache and bands disabled | 20.53 / 20.59 | 48.50 / 48.37 |
| Cache and exterior bands, general shader | 32.11 / 32.07 | 30.83 / 30.83 |
| Same cache and bands, fixed shadow shader | 59.94 / 59.88 | 12.56 / 12.58 |

The second pass ran in reverse order. With the fixed shader, 1 of 956 frame
intervals in the first run and 2 of 955 in the second spanned two refreshes.
Each of the other intervals spanned one refresh. On this list, reusing the
shadow and specializing its remaining draw each saved about 18 ms per frame.
The cost of building the first shadow texture falls outside the warm scroll.
At the same stationary list position, 156 of 1,024,000 pixels differed from
stock with shadow reuse enabled, by at most 1/255 per channel. The general and
fixed programs produced the same image at that position.

We also ran the same 16-second scroll once in the integrated interactive
build, with the fork's Linux touch and camera support enabled. It displayed
60.00 fps, with 12.54 ms of GPU time per frame. All 956 measured intervals
spanned one refresh. This run measured the photo list with the camera closed.

The cache remains opt-in in the fork. An upstream version needs a policy for
fractional-position pixel differences, eviction that respects retained rows,
release on trim, one combined clip per shadow, and device-loss handling.

### Fixed-grid coverage experiment

This branch evaluates a reusable shadow texture with two samples per axis per
device pixel. The raster stores analytic f32 coverage. The consumer decodes
four neighbors before interpolating. Fractional motion can reuse the same
texture at each position.

Two 16-second warm Pi 4 photo-list scrolls measured 53.36 and 53.17 fps,
with 17.49 and 17.50 ms of GPU time per completed flip. In the same fixture
the previous kiosk cache measured 59.94 and 59.81 fps. The
[exact-phase experiment](https://github.com/smihica/exact/commit/0b79a2860)
measured 39.08 and 40.88 fps.

The grid agreed with the analytic reference in stationary device captures
outside the cursor. At fractional positions, 17 of 40 headless comparisons
exceeded the acceptance limits on both Vulkan and GLES. Up to 651 of
98,800 pixels differed in a rounded, clipped scene, by up to 2/255. The
quality tests retain their original thresholds so this result can be reproduced.

## What to share upstream (2026-11-26)

The code comparison uses [upstream `2685f0a30`](https://github.com/expo/exact/tree/2685f0a30) and
[kiosk `f3e0473b0`](https://github.com/smihica/exact/tree/f3e0473b0). The physical measurements below
use the builds named in each experiment.

### Five candidates for separate Gists

| Item | Evidence | What to share |
| --- | --- | --- |
| 1. Submit one display frame ahead | On the Pi 4, a matched 16-second photo-list drag went from 29.94 to 60.00 displayed fps. Both fork builds used the upstream rotated final copy. The separate tap test checked which displayed row received the input. [Measurements and traces](https://github.com/smihica/exact/pull/11) | Answer the [maintainer's request](https://github.com/expo/exact/issues/145#issuecomment-7193253363) first. Include the per-frame GPU submit and flip times, the tap outcomes and the tests that keep presentation acknowledgement tied to the actual flip. |
| 2. More fixed Ink shader programs | Upstream builds eight programs at startup. The fork adds five, including one for cached shadows. Existing fork timings combine these with other rendering changes. [Upstream decision](https://github.com/expo/exact/issues/145#issuecomment-6946461526) | Compare eight programs with eight plus one candidate on the same scene. Report GPU time, cold and warm startup, draw calls and pixel parity. Measure the flat interior pass separately. |
| 3. Reuse and clip outer shadows | On an earlier fork build, shadow reuse, exterior bands and a clip shortcut together reduced headless GPU time from 49.80 to 15.10 ms at fixed list positions. [Evidence and limits](#evidence-and-its-limits) | Follow up on the [shadow Gist we offered](https://github.com/expo/exact/issues/145#issuecomment-6944035191). Use a small card list. Separate the cache and band measurements, show the pixel differences, and cover the 16 MiB budget and nested rounded clips. |
| 4a. Generate image mip levels with fewer buffers | The mip pass saves two GPU buffer allocations per level. Five image sizes, including odd and one-pixel dimensions, matched the reference filter on Pi Vulkan and GLES. [Implementation](https://github.com/smihica/exact/commit/db40ab61a) | Isolate mip generation on upstream, with allocation counts, generation time and output pixels. This change can be evaluated independently of bitmap retention. |
| 5. Skip unused paint geometry | Two early exits skip child layout reads on ordinary nodes and pattern construction on borders with no dotted or dashed edge. Main-thread CPU per displayed frame fell from 10.79 to 10.15 ms in the Pi photo list. Both builds ran near 60 fps. [Tests and measurements](https://github.com/smihica/exact/pull/9) | A small standalone diff can show both skipped operations, the zoom hit tests and the border pixel checks. |

In the first #145 reply, give item 1 three to five lines covering the result, what the tap test established,
and a link to the trace and prototype. A short note can point to item 3 when its Gist is ready.
Add each Gist here as it is published, so one index link can lead readers to the other results.
After the shadow follow-up, items 5 and 4a can stand on their own. Share item 2 once measurements
show what one variant saves and what it costs.

### Later topics

- **4b. Idle bitmap reuse.** Across eight warm flicks over previously seen photos, V3D texture-formatting submissions fell from 152 to zero. Bring that trace to a discussion of the memory budget and when to release textures. [Measurements](frame-pacing.md#fast-flicks-through-photographs) and [upstream retirement policy](https://github.com/expo/exact/commit/f4ba5552c).
- **6. Linux touchscreen scrolling.** Show the drag, fling, tap-to-stop and rubber-band behavior with a minimal list and input trace. The fork handles single-finger vertical drags. A general implementation also needs to handle nested scrolling and the reader's gesture lifecycle. Cursor visibility with mixed touch and mouse input can be reported separately. [Input limits](../KIOSK.md#limits-to-retain-during-integration).
- **7a. Linux camera and QR.** Share the use case of a live preview and barcode recognition from the same capture. The fixed Pi camera path is a working reference for the Linux capability request. [Upstream camera scope](https://github.com/expo/exact/blob/2685f0a30/rules/DEFERRED.md#L1466-L1481).
- **7b. Hatch connection on first presentation.** Reproduce the missing connection on current upstream with a frame awaiting presentation and newer input already dirty. If it reproduces, the regression test makes a small bug report independent of the camera. [Fix and test](https://github.com/smihica/exact/commit/d143342dc).

## Upstream and Pi 4 status on 2026-11-22

The fork merged upstream `9c348ab33` into the work based on `295e852a3`.
Before this merge, the fork and `kiosk` (`c45a69086`) shared upstream history
through `bba0b3a7d`. The stock-host panel test used `5f28513a4`, the revision
named in [issue #145](https://github.com/expo/exact/issues/145#issuecomment-7068414558).

| Change | Upstream status | Pi 4 evidence |
| --- | --- | --- |
| Clamp GPU limits to the adapter | Landed in `44395c82` | Previously verified with Ink on Vulkan and GLES, and Vello on Vulkan. |
| Skip full-coverage and hidden-shadow calculations | Landed in `110439978` with upstream guards | Earlier upstream A/B reduced render time at 200 ms into opening on both backends. |
| Eight fixed Ink programs at startup and general fallback | Landed in `b2a7e278c` | Both fixed-program pixel tests passed on Vulkan and both passed on GLES at `5f28513a4` with `--features wgpu/gles`. |
| Holes in masked fills with the GLES general program | The single WGSL reduction in `060b9f988` matches the earlier fork fix `76c5d695c` | The previously failing general/fixed comparison passed in the run above. |
| Output rotation and absolute input mapping | Landed in `b2a7e278c` | Earlier upstream run covered 0°, 90°, 180°, 270° and unset. New direct path exercised at 90°. |
| Repeated tap at the same point after a relative mouse click | Landed in `9985713ed` | Three confirmations at 90° with injected evdev input, including two taps with no coordinate events. |
| Find the Vulkan display through its DRM connector across separate render and display devices | Landed in `c75288253` | Stock host opened direct Vulkan on the Pi’s V3D and KMS devices. |
| Direct Vulkan presentation with GPU rotation | Landed in `5f28513a4` | Static pixels, startup, final frame and page flips checked below. VNC capture and forced fallback were tested by upstream on x86; the Pi timing run used no VNC viewer. |
| Defer clipped-row repainting and handle sticky headings separately | Independent upstream changes `e18de7e97` and `fc49f60b6` | Fork `adb9add81` was tested by comparing retained rendering with fresh paints. The same rich-list benchmark still needs a stock-upstream run. |
| Five more fixed programs, flat interiors and less shape-distance work | Fork only | Pixel and timing checks on the fork, measured with the other rendering changes. |
| Cache shadow geometry and draw exterior bands | Fork only | Shadow scenes, card lists and moving dialogs tested on the fork. Upstream’s shadow early return is a separate change. |
| Fuse rotation into the final scanout copy and allow two outstanding frames | Earlier fork experiment, retired in this integration | Four fork orientations matched reference pixels before integration. The integrated build uses upstream presentation and its single outstanding frame. |
| Downsample images without per-mip vertex and uniform buffers | Fork only | Fork mip readbacks matched the reference filter at five sizes on both Pi backends. |
| Bounded reuse of idle bitmap textures | Fork only, opt-in | List A/Bs with 32 and 160 photos, plus a ten-minute reuse-on run. See [frame pacing](frame-pacing.md#fast-flicks-through-photographs). |
| Connect hatches when a display receipt activates the session, including when newer input is dirty | Fork only | Fork regression tests and direct-display QR flow passed. |
| Live camera preview and shared luminance frames for QR | Fork only for this Linux camera path | Preview, scanner transition and camera lifecycle exercised in the fork app. |
| Linux KMS touch drag, inertia, rubber-band movement and cursor hiding | Fork only for these KMS behaviors | Touch scrolling and mixed input exercised on the fork. Upstream’s repeated-tap repair has its own row above. |
| Dialog scrim override and FPS diagnostics | Fork comparison aids | Used on the fork. The app supplies dialog motion. |

### Rich list and dialog after merging upstream

We ran the same release app and assets on the same Pi 4 with Ink/Vulkan,
90° rotation and VNC off. The order was previous fork, integrated fork,
integrated fork, previous fork. Both builds kept the fork's extra shader
programs, shadow optimizations and bitmap reuse enabled. The integration uses
upstream's display selection, GPU rotation, Vulkan presentation and completion
handling, and clipped-row rendering. It logged `direct Vulkan`.

| Physical panel, mean fps during motion | Previous fork | With upstream `9c348ab33` |
| --- | ---: | ---: |
| Drag the photo list for 16 seconds near row 20 | 59.9 | 29.8 |
| Coast after a fast flick, 12 gestures per build | 59.7 | 28.1 |
| Open / close the dialog | 60.0 / 60.0 | 30.0 / 30.0 |
| Live camera preview (30 fps source) | 30.1 | 29.9 |
| Open / close the dialog over live video | 60.0 / 57.5 | 30.0 / 30.0 |

During the two sustained drags per build, the previous fork missed 3 of
1,913 refreshes and the integration missed 963 of 1,913. The previous build
recovered its earlier pace in the last block. The Pi reported no throttling.
Across those same windows, V3D render-engine time divided by completed frames
rose from about 15.1 to 18.2 ms. Upstream's separate rotation pass and its wait
for actual presentation before the next paint both differ from the earlier
fork. These timings do not isolate their individual costs.

### Stock direct presentation on the physical panel

We built the unchanged `app.contract` from the [public dialog fixture](https://gist.github.com/smihica/43fc09c307fa2968cee057483c86c99f)
in release mode with stock upstream `5f28513a4` and its immediate parent
`c75288253`. Both used Ink/Vulkan and `EXACT_ROTATE=90` on the same
1280 × 800, 60 Hz panel. The new host logged `direct Vulkan` and stayed on
that path. The driver was Mesa 26.2.6 on V3D. The fixture enables wgpu’s GLES backend for the separate headless comparison.

- Physical scanout pixels matched at startup, with the dialog fully open,
  and after closing across all four launches. The first frame was visible before input. The final
  image remained in scanout after a further 2.2 seconds without input.
- Separately, 29 fixed-clock headless captures matched the parent byte for
  byte on each of Vulkan and GLES. These include intermediate animation frames.
- On the panel, a touch opened the dialog and a relative mouse clicked
  Confirm. The next two touches sent only a press and release at the first
  touch’s position. All three counter values matched the counter pixels from a
  touch-only reference run.

| Completed page-flip intervals after release | KMS parent mean | Stock Vulkan direct mean | Parent / direct fps |
| --- | ---: | ---: | ---: |
| Opening, 60 intervals per build | 50.0 ms | 46.7 ms | 20.0 / 21.4 |
| Closing, 33 intervals per build | 49.0 ms | 41.9 ms | 20.4 / 23.9 |

We read the active KMS framebuffer and recorded `drm_vblank_event_delivered`
on the idle kiosk during motion. There were no events after each animation settled.
The run order was direct, parent, parent, direct with four open/close cycles
per launch. The table includes the last three cycles from each launch.
The median interval was 50.0 ms in all four cases. During opening, direct
presentation reduced the first two intervals of each warm transition to two
refreshes. The remaining eight took three refreshes, as on the parent.
The startup log reported 606 ms on each timed direct launch and 458–462 ms
on the parent. VNC was disabled. The fork's [earlier near-60 Hz runs](frame-pacing.md)
used a different app scene and additional renderer changes.

## Fork implementations and earlier assessments

This table describes the fork before the November 27 integration. The
[November 27 results](#blurred-shadows-on-upstream-main-2026-11-27) identify the
implementations the fork now takes from upstream.

| Change | Implementation at the time | Value to upstream | Work before general adoption |
| --- | --- | --- | --- |
| [Shader specialization](../host/ink/src/device/wgpu/programs.rs) | Upstream compiles eight material programs at startup. The fork adds five material programs and a flat fill for the interior of large solid shapes. | The original eight landed in `b2a7e278c`. The additions are candidates for measurement on other GPUs. | Compare startup, frame times and pixel parity before expanding the fixed set. |
| [Idle bitmap reuse](../host/painted/src/ink/images.rs) | An opt-in cache keeps textures for still-owned decodes up to 8 MiB or 64 idle entries per painter. | Repeated photos in a virtual list can skip uploading the pixels and rebuilding mips. | Upstream chose immediate retirement in `f4ba5552c`. Compare reuse and memory costs on another host before proposing a default. See the [fast-flick results](frame-pacing.md#fast-flicks-through-photographs). |
| [Direct output](../host/ink/src/device/wgpu/scanout.rs) | The integrated fork uses upstream's Vulkan DRM surface, presentation IDs and completion waits. The host can mirror to VNC or repaint via ordinary KMS after a failed presentation. | Landed in upstream and replaced the fork's original direct path. | VNC capture and forced fallback on the physical Pi are open checks. |
| [GPU rotation](../host/linux/src/rotation.rs) | The integrated build uses upstream's GPU rotation and absolute input mapping. The earlier fork fused rotation into its final presentation copy. | GPU rotation landed in `b2a7e278c` and works with upstream direct presentation. | Measure any future fused-copy candidate against the upstream host on the same workload. |
| [GPU limits](../host/ink/src/device/wgpu.rs) | Use upstream `requested_limits` in Ink and [Vello](../host/painted/src/gpu.rs); clamp defaults to adapter capacities and validate renderer requirements. | Already incorporated upstream in `44395c82`. | Confirmed on Pi 4 with Ink/Vulkan, Ink/GLES and Vello/Vulkan; the fork uses the upstream implementation. |
| [Split render/display devices](../vendor/wgpu-hal/src/vulkan/drm.rs) | Match the Pi's display connector to a different rendering device. | Integrated in Exact’s vendored wgpu in `c75288253`; also relevant to gfx-rs/wgpu. | The integrated fork uses the upstream [vendor changes 18 and 19](../vendor/wgpu-hal/EXACT-PATCHES.md). |
| [Shadow cache](../host/painted/src/ink/shadows.rs) | Raster eligible fixed geometry once, then reuse alpha with live position, color and opacity. | High: useful for repeated cards and moving dialogs. | Evaluate cache lifetime, changing geometry, allocation failure and first-use work. The current cache has a 16 MiB non-evicting budget and falls back to analytic drawing for uncached geometry after the budget is exhausted. |
| [Outer shadow bands](../host/painted/src/ink/shadows.rs) | Draw the visible bands around the excluded border box. | Useful: reduces the area drawn for outer shadows. | Fix the extra combined-mask work with nested rounded clips or fractional rectangular clips, then measure broader scenes. |
| [Display hatch activation](../host/painted/src/presenter/display_frame.rs) | Connect hatches when a successful display receipt activates the session, including when newer input is dirty. | Correctness fix: real display sessions can otherwise omit hatches that work under the headless agent. | Regression tests cover first display, one-time connection and a stale receipt after reload; the Pi direct-display QR fixture also reaches its next screen. |
| [Camera preview](../host/painted/src/image/kiosk_camera.rs) | Read and convert each fixed-size frame from `rpicam-vid`; publish only the latest through the Linux image path. Bounded luminance subscribers let apps analyze that same capture. | Useful as a working Pi reference and evidence for Linux camera support. | Fit the capture/preview lifecycle into `@exact/media-capture` and `@exact/camera`, including grants and agent substitutes. Linux camera support remains deferred in the [pinned upstream rules](../rules/DEFERRED.md); widening that scope needs a maintainer decision. |
| [Touch drag and inertia](../host/linux/src/display/kiosk_scroll.rs) | Single-finger vertical drag, fling, cancellation of a pending tap and catching an active fling. | High for a usable Linux touchscreen host. | Connect to the existing reader drag lifecycle and completion/events; define nested-scroller behavior and device classification. |
| [Rubber-band overscroll](../host/linux/src/display/kiosk_scroll.rs) | Resist movement beyond the boundary and spring back; [painting](../host/painted/src/content_region/presenter.rs) keeps visual displacement separate from logical scroll. | Useful alongside touch scrolling. | Fit the platform's behavior and motion preferences. The probe's physics were chosen for comparison with egui. |
| [Touch cursor behavior](../host/linux/src/display/pointer.rs) | Hide the cursor for touch; reveal it on mouse movement. | High, and relatively independent of renderer changes. | Build on the existing [Pointing and touch_only classification](../host/painted/src/presenter/device.rs) to distinguish direct touch from pen/touchpad reports, and handle initial mouse visibility. |
| [Dialog backdrop fade](../host/painted/src/paint/alert.rs) | Under `KIOSK_DIALOG_FADE=1`, replace the scrim with black at `72% × presented dialog opacity`. | Low for this patch as written; useful as a backdrop styling/animation use case. | Put the application's color, opacity and transition choices at the authoring boundary. The fixed black 72% value exists to match the comparison app. |
| Virtual-list comparison | An external app combines images, SVG, Japanese text, rounded corners, shadows and append batches using Exact's existing virtual list. | High as a benchmark or sample, not as a new list implementation. | Publish a small self-contained fixture with matching input and capture procedures. The list and virtualization primitives already belong to upstream. |
| [FPS overlay](../host/painted/src/paint/kiosk.rs) and [timing](../host/linux/src/display/event_loop.rs) | Count attempts to draw and log timing for local comparisons. A frame may still be discarded. | Useful measurements; low value for the dedicated overlay as written. | Use existing performance tooling where possible. The benchmark measures displayed fps from completed DRM page flips. |

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
  GPU-limit selection code. The status table above records subsequent decisions.
- The [two general early returns were accepted](https://github.com/expo/exact/issues/145#issuecomment-6799709624)
  in [`11043997`](https://github.com/expo/exact/commit/110439978ef35965e79e5842986413fed19a79a7): full coverage deep inside a shape, and skipping the shadow
  integral under a fully covered border box. Upstream rewrote the guards and
  added boundary tests. The fork's merge uses those upstream guards. These are
  already integrated, rather than pending local proposals.
- The fork's [Pi retest](https://github.com/expo/exact/issues/145#issuecomment-6814656945)
  confirms the device-creation failure without the limits adjustment and the
  remaining register spills after the early returns. At the time of that retest, specialization was
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

## Initial integration (2026-11-12)

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
commits not yet integrated into `kiosk`. At the initial snapshot the fork delta was
34 files, +2,365/-127 lines, before adding this memo.

A useful next report would pair the shadow-cache and outer-band results with a
small public list fixture. One hypothesis for the main-merge regression is the
change to `shape_coverage` guards. Compare the old and upstream guards with the
host and fixture held constant to isolate that effect before drawing a conclusion.

## Main integration (2026-11-13)

The painted host is now `exact-painted`; Linux owns KMS, input and the display
loop. Renderer, camera and presenter experiments follow that extraction.
`presenter/kiosk.rs` exposes the display and gesture operations the Linux shell
needs without exposing the painter. A replaced tree clears visual overscroll.
The split-device wgpu-hal patch is numbered 18 after upstream's new vendor patches.

Upstream `44395c82` creates the device and renders correctly on Pi 4 for
Ink/Vulkan, Ink/GLES and Vello/Vulkan, with an unmodified SDK. The requested
[confirmation is posted in #145](https://github.com/expo/exact/issues/145#issuecomment-6860673503).

The integration was compared with `aa881225c` on Pi 4 / Mesa 26.2.6, release
builds, 800×1280. Both runs enabled specialization, shadow caching and outer
bands. GPU times are V3D `drm-engine-render` deltas, medians of ten warmed
samples: the dialog at 200 ms and successive fixed list positions. The 52
paired PNGs matched byte-for-byte within each backend.

| Scene / backend | Before | After |
| --- | ---: | ---: |
| Dialog / Vulkan | 15.59 ms | 15.61 ms |
| Dialog / GLES | 12.24 ms | 12.16 ms |
| List / Vulkan | 16.48 ms | 16.53 ms |
| List / GLES | 13.52 ms | 13.52 ms |

Direct Vulkan output, portrait rotation and injected touch scrolling also ran.
Four 1.6-second observation windows, each starting with a roughly 0.6-second
swipe, gave 48.55 fps before and 43.11 fps after;
reversing the order gave 48.77 fps before and 45.20 fps after. FPS is the inverse
of the mean page-flip interval within the swipe windows. Temperatures stayed at
31.1–34.0°C with `throttled=0x0`. The small sample shows a display-frame regression
worth profiling separately: unchanged offscreen GPU times do not identify its
cause. Camera preview also captured frames through the shared painted host.

Validation: Ink 38 passed, Linux 51 passed; painted 2,033 passed and five failed.
All five failures also reproduced on unmodified upstream `221e551bf`: mouse
back/forward history, two retained inline-row cases, sheet closing and bilingual
text ordering. That baseline had two additional failures in watch/haptic timing.
Scoped Clippy with scanout, formatting, caps and boot checks passed. Review used
GPT-6 Astra and Opus 5.5 with an independent Claude subagent.
