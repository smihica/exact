# A tvOS build bundles a Canvas 2D GPU module whose every shader is empty

**Status:** Closed
**Resolution:** tvOS device and simulator modules carry compiled Metal kernels; unsupported Apple targets build as ABI-0 stubs.
**Systems:** canvas/vello/build.rs, host/apple/build.mjs, host/apple/Sources/ExactSurfaces/Canvas2DImage.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`canvas/vello/build.rs` picks the SDK its kernels are compiled for by the target's suffix (`:88-96`):

```rust
let sdk = if target.ends_with("apple-ios-sim") { Some(("iphonesimulator", …)) }
    else if target.ends_with("apple-ios") { Some(("iphoneos", …)) }
    else if target.ends_with("apple-darwin") { Some(("macosx", …)) }
    else { None };
```

`aarch64-apple-tvos-sim` and `aarch64-apple-tvos` match none, and with `None` each kernel's Metal library is `Vec::new()` (`:162`). The crate is not built as the stub either: `exact_no_metal_toolchain` is set only when the Metal toolchain is missing (`:79`), so `ecg_abi()` answers 3 (`canvas/vello/src/lib.rs:194-200`) and the host takes it for a working module.

`host/apple/build.mjs` builds the module for every Apple target when the toolchain is installed and bundles it (`:898`, `:1002`; `host/apple/bundle.mjs:175`). Nothing excepts `--tvos`. LLP 1008 §9 "tvOS" says the Metal GPU path takes the iOS behaviour there.

## The failure

```
bun host/apple/build.mjs --tvos hello-apple --no-install
```

Observed: the bundle's `Frameworks/libexact_canvas_gpu.dylib` holds 22 kernels, each a zero-length Metal library. Such a module loads and makes canvases, and every replay fails. When a canvas that animates gets a fresh bitmap, the host gives its first lists to the module; the replay answers 2, the host logs `canvas gpu: canvas N failed; Core Graphics draws its next bitmap` and returns without keeping those lists (`Canvas2DImage.swift:274-304`). What that first draw drew is lost: a canvas that repaints each frame misses one frame, and one that draws on top of its last picture never shows it. No canvas draws on the GPU.

Expected: kernels compiled for `appletvsimulator` and `appletvos`, or no module in a tvOS bundle, so Core Graphics draws from the first list.

## Verification

Run: the command above exits 0 and its bundle holds `Frameworks/libexact_canvas_gpu.dylib` (4.2 MB, platform `TVOSSIMULATOR`, exporting `_ecg_abi`), built from a `kernels.bin` of 0 bytes and a `kernels.rs` of 22 kernels, each `range: (0, 0)`, with no `.metallib` beside them. The Mac build's module is 232 KB larger and holds 22 more `MTLB` headers.

The module's behaviour was run on macOS, on a Mac's GPU, with the build script's `apple-darwin` arm disabled so that it takes the same `None` arm. A temporary test in `canvas/vello/tests` made a 64 by 48 canvas and replayed one `fillRect` three times:

```
exact canvas gpu: Validation Error … create_shader_module_passthrough, label = 'vello.fine_area' … Failed to generate the backend-specific code
exact canvas gpu: Validation Error … create_compute_pipeline … ShaderModule with 'vello.fine_area' label is invalid     (so for each kernel)
ecg_canvas_new null = false
exact canvas gpu: wgpu: Validation Error … set_pipeline … ComputePipeline with 'vello.pathtag_reduce' label is invalid
round 0: ecg_canvas_replay = 2; pixel (10,10) bgra = [0, 0, 0, 0]                                                (rounds 1 and 2 the same)
```

With the real kernels the same test printed `ecg_canvas_replay = 0` and the fill's pixel. Not run: a canvas on an Apple TV simulator. What the host does with the failed replay is read.

## Direction

Add the two tvOS targets to the build script's table (`appletvsimulator` with `-mtvos-simulator-version-min`, `appletvos` with `-mtvos-version-min`, as `svgFilterLibrary` in `build.mjs:156` names them), and make the `None` arm the stub (`exact_no_metal_toolchain`), so an unknown Apple target never yields a module without shaders.

## Related

`QUEUE.md`, "tvOS, what is owed", lists the driver and the lanes, not this.
