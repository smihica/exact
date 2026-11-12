#[path = "../metal.rs"]
mod metal;

#[test]
fn apple_targets_select_their_metal_sdk() {
    for (target, sdk, minimum) in [
        (
            "aarch64-apple-tvos-sim",
            "appletvsimulator",
            "-mtvos-simulator-version-min=17.0",
        ),
        ("aarch64-apple-tvos", "appletvos", "-mtvos-version-min=17.0"),
        (
            "aarch64-apple-ios-sim",
            "iphonesimulator",
            "-mios-simulator-version-min=17.0",
        ),
        ("aarch64-apple-ios", "iphoneos", "-mios-version-min=17.0"),
        (
            "aarch64-apple-darwin",
            "macosx",
            "-mmacosx-version-min=14.0",
        ),
        ("x86_64-apple-darwin", "macosx", "-mmacosx-version-min=14.0"),
    ] {
        assert_eq!(metal::sdk(target), Some((sdk, minimum)), "{target}");
    }
    for target in ["aarch64-apple-watchos", "aarch64-apple-visionos", "unknown"] {
        assert_eq!(metal::sdk(target), None, "{target}");
    }
}

#[test]
#[cfg(target_vendor = "apple")]
fn an_advertised_module_contains_compiled_kernels() {
    use exact_canvas_vello::{ecg_abi, kernels::KERNELS};

    if ecg_abi() == 0 {
        assert!(KERNELS.is_empty());
        return;
    }
    assert!(!KERNELS.is_empty());
    for kernel in KERNELS {
        assert!(
            kernel.precompiled().metallib.starts_with(b"MTLB"),
            "{} has no compiled Metal library",
            kernel.name
        );
    }
}
