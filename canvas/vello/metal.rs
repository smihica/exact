pub fn sdk(target: &str) -> Option<(&'static str, &'static str)> {
    if target.ends_with("apple-ios-sim") {
        Some(("iphonesimulator", "-mios-simulator-version-min=17.0"))
    } else if target.ends_with("apple-ios") {
        Some(("iphoneos", "-mios-version-min=17.0"))
    } else if target.ends_with("apple-tvos-sim") {
        Some(("appletvsimulator", "-mtvos-simulator-version-min=17.0"))
    } else if target.ends_with("apple-tvos") {
        Some(("appletvos", "-mtvos-version-min=17.0"))
    } else if target.ends_with("apple-darwin") {
        Some(("macosx", "-mmacosx-version-min=14.0"))
    } else {
        None
    }
}
