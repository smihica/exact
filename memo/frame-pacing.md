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

## Fast flicks through photographs

A 650 px flick over 100 ms exposed occasional 33 ms display intervals even
when slower drags were close to 60 fps. When a row left the visible set and
lost its pin, Ink retired its bitmap texture. Showing the same decoded photo
again uploaded its pixels and regenerated its mip chain.

The opt-in bitmap cache keeps the uploaded texture while the decoded bitmap
has another owner. When making room it evicts textures in the order they
became idle.
The size and count limits apply to idle entries per painter. Visible textures,
retained rows, driver alignment and storage awaiting GPU completion have
separate costs. A first encounter with a photo still pays for upload and mips.

The release app used 32 different 256 × 192 photos drawn at 144 × 108 in
virtual rows with a shadow, icon and text. Each setting ran in two processes,
six flicks per process, in off/on/on/off order. We counted missed refreshes
from DRM page-flip sequence numbers in the 500 ms after each release. The
last four flicks per process revisited photos drawn earlier.

| Final release, 32 photos | Cache off | Cache on |
| --- | ---: | ---: |
| Missed refreshes over 12 flicks | 10 / 347 | 6 / 347 |
| Missed refreshes over the eight warm flicks | 8 / 231 | 2 / 231 |
| V3D texture-formatting submissions over those eight flicks | 152 | 0 |

An earlier run of the opt-in implementation missed 17 versus 2 refreshes
overall and 12 versus 0 after warm-up. In the final release the first two
flicks of each process missed 2 refreshes in total with reuse off and 4
with it on. The reliable
difference across both runs was the absence of repeated texture work on
warm flicks. Occasional missed refreshes remained.

With 160 distinct image sources, 24 forward flicks per setting missed 33
refreshes with the cache off and 21 with it on.
Median V3D resident memory at the last scrolling sample of the two runs
per setting was 66.85 MiB off and 66.93 MiB on. The driver accounts for
other GPU allocations in these figures. In a separate ten-minute run with
reuse enabled, 531 flicks alternated directions every 12 gestures. Over the
last five minutes, V3D resident memory ranged from 67.63 to 67.76 MiB.
Anonymous proportional set size ranged from 60.67 to 60.71 MiB.

All 74 fixed-time list and dialog PNGs matched with reuse on and off across
Vulkan and GLES. Six more PNGs covering changes of scale, rotation and clip
matched the original renderer. Six frames covering removal and redisplay of
the photos also matched with reuse on and off. On the direct display a camera preview
updated at 30 fps and the dialog over the preview opened and closed at 60 fps.
