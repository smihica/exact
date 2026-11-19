# Ink frame pacing on a portrait display

The same release app and animated scenes were run against the fork's
[`c45a69086`](https://github.com/smihica/exact/commit/c45a69086b7121c5c4fe90fde1fc504ef6fa4e11)
and this change on a Raspberry Pi 4. Both used the Vulkan direct path on a
60 Hz, 1280 × 800 panel with an 800 × 1280 portrait UI. The fork's shadow
texture cache and exterior bands were enabled in both builds.

| Gesture or transition | Baseline (fps) | Changed (fps) |
| --- | ---: | ---: |
| Drag through rich list rows | 15.3–15.4 | 59.7–60.0 |
| Coast after release | 15.4–15.6 | 59.0–59.7 |
| Continuous drag near row 20, 16 s | 15.5 | 59.9 |
| Open the dialog and fade in its scrim | 28.1–28.5 | 60.0 |
| Close the dialog and fade out its scrim | 31.7–32.3 | 60.0 |
| Open / close over a live camera preview | 19.7 / 20.0 | 60.0 / 60.0 |

FPS is the reciprocal of the mean interval between completed DRM page flips
during motion. List ranges cover two baseline runs and three changed runs. In the 16-second changed run, 952 of 954 intervals took one
refresh, and two took two. The 95th percentile fell from 66.7 to 16.7 ms.
Warm tap-release to the first observable change of a background pixel fell
from 51.0 to 18.6 ms on open and from 107.1 to 39.9 ms on close (medians of
five taps, sampled from the active framebuffer). This measures scanout
availability; panel response and touch-controller latency are additional.

A launch with empty application and Mesa caches took 10.58 s on the baseline and
12.76 s after the change. The next launch with those caches took 0.58 s and
0.64 s respectively (one cold/warm pair per build). These startup-compiled programs
trade first-launch time for lower frame times.

## Workload

- Populate a virtual list with 200 rows, each 184 logical pixels high, using
  images, an SVG, three lines of Japanese text, a rounded card and a shadow.
  Scroll 650 px over 600 ms in each direction at starting rows 0, 10 and 20.
  Measure drag and the next 500 ms of inertia separately. For continuous
  motion, send absolute touch positions at 120 Hz, reversing every 700 ms.
- Place a rounded, shadowed dialog above a full-screen dimming layer. Animate
  its translation and opacity for 400 ms on open and 160 ms on close. Tap
  through four cycles, aggregate the three warm cycles and take intervals
  between animation frames after release. Press feedback and the 80 ms
  synthetic finger hold precede that interval. One changed run recorded a
  shorter last transition; both repeat runs recorded its full duration.
- For the camera scene, leave a 640 × 480, 30 fps camera streaming behind
  the dialog and operate it on the same screen. Count flips in the 400 ms
  open or 160 ms close window following release. With just the preview, the
  display updated at 29.9–30.0 fps. Capture continued at 30 fps while the
  overlay animated at the panel refresh rate.

## Where the time went

- Ink uses fixed programs for common material and clip combinations and keeps
  a general fallback. A cached shadow samples its existing alpha while its
  live clip still excludes the border box. A conservative rectangular region
  in the shape shader shares work between its three distance samples.
- Large, axis-aligned, solid fills draw their fully covered interior with a
  flat-color program and their perimeter with the original edge shader.
  Scissors are disjoint, including for translucent fills. Small interiors,
  transformed edges and unsupported flags keep their normal rendering.
- A retained row wholly outside its clip can keep its recording and input
  metadata until visible. Visible rows still revalidate their actual origin,
  including fractional glyph placement. Sticky descendants use regular
  painting because they may remain visible outside the row.
- Direct presentation rotates in the final integer texel copy. FIFO permits
  two outstanding frames so the next render can overlap presentation.
  A full FIFO queue can add one refresh to latency for a light scene. Readback
  keeps the existing rotation path.
- Each image mip level now reads only the previous level and draws directly
  into the next. This removes two GPU buffers per level. Even source
  dimensions allow an exact four-texel average; odd sizes and single-pixel
  axes use the existing area filter. The pipeline is shared by all mipmapped
  images, including video frames.

## Checks

Compare fixed and general programs on fractional geometry, rounded corners,
intersecting clips and translucent overlap. Compare successive frames from a
scrolling presenter against a newly painted one, including sticky content.
For the app, compare 29 fixed-time dialog frames and eight list frames per
backend. All 74 PNGs matched the corresponding baseline PNG byte for byte.
The generated mip levels also matched their reference area draw for patterned,
translucent images at five sizes on both Pi backends. For direct scanout, all four orientations matched an independently rotated
logical capture at every pixel.

On this Pi, the existing shader coverage test reports a 3/255 alpha deviation
for the `corners-2` sample on both Vulkan and GLES. The baseline reports the
same value at the same pixel. Comparing the complete RGBA output of all 12
coverage scenes against the baseline gave byte-identical results on each Pi
backend, including the cases after that assertion.
