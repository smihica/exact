# Linux kiosk experiments

This fork evaluates Exact on a Raspberry Pi 4 touchscreen kiosk. It carries
experimental Linux display, input and camera changes alongside Ink performance
work. The starting revision is `fe4632ed1800ce58f3bcfa880e95f834395d77d9`.

The [kiosk change memo](memo/kiosk-upstream.md) maps each experiment to its code,
assesses its value to upstream, and records the measurements and open questions.

## Branches and review

- `main` follows `expo/exact:main` and contains no kiosk changes.
- `kiosk` is the integration branch. Work on a topic branch and open a pull
  request to **`smihica/exact:kiosk`**, never to `expo/exact`.
- Merge upstream into a topic branch, resolve overlapping implementations,
  then run the affected tests and device comparisons before integration.
  Prefer an equivalent upstream implementation and remove the replaced local
  path. A clean text merge alone does not establish that behavior is preserved.
- Follow `AGENTS.md`, `rules/RULES.md` and `docs/testing.md`. Keep commits focused
  and retain the reason for a platform-specific branch. Do not invent LLP
  references for experiments that have no upstream design record.
- Keep device access details, credentials, organization-specific assets and
  camera captures out of this public repository and its commit history.
  Upstream feedback is shared through separately reviewed issues.

## Runtime switches

Set `EXACT_PAINTER=ink` and `WGPU_BACKEND=vulkan` for Linux direct
presentation. Select the display device with `EXACT_DRM` to use the direct
host even when launching from a Wayland session. The host reports
`direct Vulkan` at startup when the driver supports presentation IDs and
waits. On failure it reports the reason and uses KMS dumb buffers.
Existing app manifests can keep their `scanout` feature.
Use `EXACT_ROTATE=0|90|180|270` to orient the picture and absolute input.
Older launchers can still select 90 degrees with `KIOSK_PORTRAIT=1`. GLES comparisons use `WGPU_BACKEND=gl` in a build
that enables wgpu's GLES backend; the direct display path is Vulkan-only.
Ink compiles thirteen `fs_main` material programs (the twelve upstream classes
plus a `CLIP1`-only outer-shadow class) alongside the dedicated `fs_flat_fill`
and `fs_fast_shadow` pipelines at device creation.
Large unclipped solid fills use `fs_flat_fill` for their interior, and large
analytic outer shadows skip their excluded `CLIP1` interior via scissor strips
without allocating any shadow cache texture. Eligible uniform-scale, non-skewed
`CLIP1`-only outer shadows automatically use `fs_fast_shadow`; all other shadows
use the standard 8-tap `fs_main` shader.
The direct Vulkan host rotates on the GPU. With three swapchain images it can
submit one frame while the previous frame awaits presentation. Hit testing
and animation starts follow the completed presentation. On a Pi 4 with a
60 Hz portrait panel, a 16-second photo-list drag measured 30.0 fps with a
single pending frame and 59.8 fps with one frame ahead and zero-cache analytic
outer shadows. Measurements are recorded in the
[kiosk memo](memo/kiosk-upstream.md#zero-cache-analytic-outer-shadows-on-upstream-main-2026-11-28).

The other experimental switches are `KIOSK_TOUCH_SCROLL=1`,
`KIOSK_DIALOG_FADE=1` and `KIOSK_CAMERA=1`. `KIOSK_FPS=1` adds an on-screen
counter; `KIOSK_TIMING=1` logs frame submission timings. The retired
`KIOSK_SHADOW_CACHE` and `KIOSK_SHADOW_BANDS` switches are no longer needed and
have been removed; see the
[zero-cache Pi 4 measurements](memo/kiosk-upstream.md#zero-cache-analytic-outer-shadows-on-upstream-main-2026-11-28)
and the historical
[cached shadow comparison](memo/kiosk-upstream.md#blurred-shadows-on-upstream-main-2026-11-27).

Ink reuses full-size photo textures by default. Each painter keeps up to 64
idle textures within an 8 MiB budget including mip levels. Memory trimming
releases idle entries.

Camera capture runs while an image using `kiosk-camera:preview` is mounted.
The probe reads tightly packed 640×480 YUV420 at 30fps, converts it to RGBA,
then rotates it clockwise into a 480×640 preview. It is a fixed Pi capture
path, not a Linux implementation of the cross-platform camera package.
`exact_painted::image::kiosk_camera::frames()` lets an app analyze the same stream, for example
for QR recognition. Each subscriber has one queued unrotated luminance frame;
a slow reader drops incoming frames, and dropping the receiver unsubscribes.
Subscriptions do not start capture or change preview ownership.

## Limits to retain during integration

- Cursor hiding currently classifies BTN_TOUCH devices together, including
  pen tablets. Touchpads use upstream relative motion handling. A mouse cursor
  appears after its first motion.
  The shell does not yet forward the device's pointer kind to the presenter.
- Touch scrolling uses immediate scroll updates rather than the reader's
  drag lifecycle; scroll-drag events, reader completion and pull-to-refresh
  are not wired.
  It is single-finger and vertical. It is not a general gesture
  recognizer for every Linux input device or nested scroll hierarchy.
- During signed update activation, image metadata preparation can time out if
  the candidate's initial view mounts the live camera. The camera source never
  acquires the packaged-image metadata that this check waits for.
- The Linux DRM host does not signal memory pressure, window hiding or focus
  loss. Idle textures stay within the budget. The next frame retires entries
  whose decoded picture has been released.
- `fs_fast_shadow` is restricted to uniform-scale (`0 < sx <= 64`, `sigma * sx >= 1`),
  non-skewed, `CLIP1`-only outer shadows with `0.5 <= sigma < 32768`,
  `r_max <= 2.0 * sigma`, `min(w, h) >= 2 * r_max + 7.5 * sigma`, and a device-space
  `clip1` box with `w, h >= 4` and all corner radii in `[1.5, min(w, h) / 2]`.
  Within this domain its alpha stays within `0.61/255` of an exact 2D Gaussian;
  shadows outside it automatically fall back to the stock 8-tap `fs_main` path
  with no extra VRAM allocation.
- Because upstream `e2524d34d` enables desktop overlay surfaces on Linux
  (`serve_overlays()`), `KIOSK_DIALOG_FADE=1` applies to plain `<dialog>`
  elements (`paint_alert`), not desktop confirmation overlays
  (`role="alertdialog"`), whose scrollable bodies rely on wheel input rather
  than `KIOSK_TOUCH_SCROLL=1`.

## Focused checks

Use the repository's pinned Rust and Bun toolchains and its normal dependency
setup. The Linux and Ink suites cover the affected renderer and host paths:

```sh
cargo test --locked -p exact-linux --lib kiosk
cargo test --locked -p exact-painted --lib kiosk
cargo test --locked -p exact-painted --lib ink::
cargo test --locked -p exact-ink --lib
cargo clippy --locked -p exact-linux -p exact-painted -p exact-ink --lib --tests -- -D warnings
```

Run `bun scripts/affected.mjs` for the full affected suite list. Compare rendered
pixels at fractional positions, clipped and translucent scenes, and retained-row
movement. Measure real display page flips separately from headless GPU rendering.
Hardware checks also cover camera start/stop/restart, portrait touch coordinates,
scroll inertia and dialog transitions.
