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

Build the consuming app with the Linux host's `scanout` feature for direct
presentation. Set `EXACT_PAINTER=ink`, `WGPU_BACKEND=vulkan` and
`KIOSK_DIRECT=1`; select the display device with `EXACT_DRM`.
`KIOSK_PORTRAIT=1` maps a landscape panel to a portrait UI and rotates absolute
touch coordinates with it. GLES comparisons use `WGPU_BACKEND=gl` in a build
that enables wgpu's GLES backend; the direct display path is Vulkan-only.
Setting `EXACT_INK_GENERIC` to any value disables shader specialization for
comparisons.

The other experimental switches are `KIOSK_TOUCH_SCROLL=1`,
`KIOSK_DIALOG_FADE=1`, `KIOSK_CAMERA=1`, `KIOSK_SHADOW_CACHE=1` and
`KIOSK_SHADOW_BANDS=1`. `KIOSK_FPS=1` adds an on-screen counter;
`KIOSK_TIMING=1` logs frame submission timings.

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
- Shadow textures share a 16 MiB non-evicting budget. Animated dimensions,
  scale or blur create new geometries; after the budget is reached, new shapes
  use analytic drawing. Texture allocation failure still uses the renderer's
  existing error handling.
- Outer bands preserve the live border-box exclusion, but two parent rounded
  clips can repeat the combined-mask fallback for each band. Fractional-edge
  rectangular clips can also take that path. Keep this optimization opt-in
  until splitting happens after clip handling, including retained-row replay.
- Direct presentation currently completes in the painter's finish path, so a
  pixel-only request on that attached painter would present too. Its internal
  boot/flip measurements also lack Vulkan presentation-completion events; use
  DRM page flips for device measurements.
- New shader classes and shadow geometries incur first-use work. Warm-frame
  throughput does not measure that cost or long-running stability.

## Focused checks

Use the repository's pinned Rust and Bun toolchains and its normal dependency
setup. The Linux and Ink suites cover the affected renderer and host paths:

```sh
cargo test --locked -p exact-linux --lib kiosk
cargo test --locked -p exact-painted --lib kiosk
cargo test --locked -p exact-painted --lib ink::
cargo test --locked -p exact-ink --lib
cargo clippy --locked -p exact-linux -p exact-painted -p exact-ink --lib --tests --features exact-linux/scanout -- -D warnings
```

Run `bun scripts/affected.mjs` for the full affected suite list. Compare rendered
pixels at fractional positions, clipped and translucent scenes, and retained-row
movement. Measure real display page flips separately from headless GPU rendering.
Hardware checks also cover camera start/stop/restart, portrait touch coordinates,
scroll inertia and dialog transitions.
