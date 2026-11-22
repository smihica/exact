//! LLP 1047.001 D5 on a reload: an archive whose linked set is hello's (no
//! family) refuses a plan that uses a family before anything changes, and the
//! running plan stays; it refuses an entry's boot the same way. This proves admission, not what the archive omits: the
//! test binary carries every family. Its own process, since an archive's linked set is the
//! process's (`exact_apple::link::set`); `reload_admitted.rs` is the
//! all-linked case.

use exact_apple::abi::{Bridge, Hooks};
use exact_apple::link::{self, Uses};

#[path = "reload/common.rs"]
mod common;
use common::{
    boot, boot_entry, digest, entry, media, plan, source, NoData, ACTIVITIES, ANIMATED, SEARCH,
    TOOLS, TRACING,
};

#[test]
fn hellos_binary_refuses_a_reload_into_a_family_and_keeps_its_plan() {
    link::set(Uses::NONE);
    let mut bridge = Bridge::<NoData>::new();
    boot(&mut bridge, &plan("text \"Hello world\""));
    let before = digest(&mut bridge);
    for (next, names) in [
        (plan("text \"a\" filter=\"blur(2px)\""), "effects"),
        (
            plan("svg viewBox=\"0 0 10 10\"\n      circle cx=5 cy=5 r=4"),
            "effects, svg",
        ),
        // An SVG `data:` image, which only its bytes say is one.
        (
            plan("image \"data:image/svg+xml,%3Csvg%2F%3E\""),
            "effects, svg, images",
        ),
        (plan("view hatch=\"avatar\""), "native_views"),
        (plan("input type=\"checkbox\""), "controls"),
        (
            plan("row role=\"tablist\"\n      button \"a\" role=\"tab\""),
            "controls",
        ),
        (
            plan("column sharedElement=\"hero\"\n      text \"a\""),
            "transitions, flights",
        ),
        (
            plan("column position=\"sticky\" top=0\n      text \"a\""),
            "sticky",
        ),
        (source(ACTIVITIES), "activities, widgets"),
        (
            plan("column background-image=\"linear-gradient(red, blue)\""),
            "gradients",
        ),
        (plan("input type=\"file\" id=\"f\""), "picker"),
        (source(TRACING), "traces"),
        (plan("text \"a\" wakelock=\"screen\""), "wake_lock"),
        (plan("text \"a\" user-select=\"text\""), "text_selection"),
        (plan("text \"**a**\" markup=\"markdown\""), "markdown"),
        (source(ANIMATED), "animations"),
        (
            plan("column navigationKey=\"a\"\n      text \"a\""),
            "navigation",
        ),
        (
            plan("column navigationKey=\"a\"\n      column navigationKey=\"b\" navigationPresentation=\"modal\"\n        text \"a\""),
            "navigation",
        ),
        (
            plan("column navigationKey=\"a\"\n      column navigationKey=\"a\" navigationBar=\"none\"\n        text \"a\""),
            "navigation",
        ),
        (
            plan("column navigationKey=\"a\" navigationHistory=\"back\"\n      text \"a\""),
            "navigation",
        ),
        (plan("canvas surface=dot() width=40 height=40"), "surfaces"),
        (source(TOOLS), "tools"),
        (source(SEARCH), "content_index"),
    ]
    .into_iter()
    .chain(media())
    {
        let len = bridge.input_write(&next);
        let out = bridge.prepare_plan(len, NoData, Hooks::none(), 390.0, 844.0);
        let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
        assert!(
            reply.contains(&format!("Unlinked(\\\"{names}\\\")")),
            "{names}: {reply}"
        );
        let out = bridge.commit_plan();
        let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
        assert!(
            reply.contains("error"),
            "a refused candidate is not committed: {reply}"
        );
        assert_eq!(digest(&mut bridge), before, "{names}");
    }
}

/// A `dialog` is the dialogs family's on iOS and tvOS, and on the Mac the
/// core's top layer presents it, so hello's binary takes it there.
#[test]
fn a_dialog_is_refused_only_where_its_family_presents_it() {
    link::set(Uses::NONE);
    let mut bridge = Bridge::<NoData>::new();
    boot(&mut bridge, &plan("text \"Hello world\""));
    let next = plan("dialog id=\"d\"\n      text \"a\"");
    let len = bridge.input_write(&next);
    let out = bridge.prepare_plan(len, NoData, Hooks::none(), 390.0, 844.0);
    let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
    let mac = cfg!(target_os = "macos");
    assert_eq!(reply.contains("Unlinked(\\\"dialogs\\\")"), !mac, "{reply}");
    let before = digest(&mut bridge);
    let out = bridge.commit_plan();
    let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
    assert!(!reply.contains("Unlinked"), "{reply}");
    assert!(
        mac || reply.contains("error"),
        "a refused candidate is not committed: {reply}"
    );
    assert_eq!(
        digest(&mut bridge) != before,
        mac,
        "the Mac commits it; elsewhere hello's plan stays"
    );
}

/// A first boot of a plan that uses a family is refused too.
#[test]
fn hellos_binary_refuses_to_boot_a_media_or_canvas_plan() {
    link::set(Uses::NONE);
    let canvas = (plan("canvas surface=dot() width=40 height=40"), "surfaces");
    for (plan, name) in media().into_iter().chain([canvas]) {
        let mut bridge = Bridge::<NoData>::new();
        let out = bridge.boot(&plan, NoData, Hooks::none(), 390.0, 844.0);
        let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
        assert!(
            reply.contains(&format!("Unlinked(\\\"{name}\\\")")),
            "{name}: {reply}"
        );
    }
}

/// An entry boots only where the widgets family is linked: on hello's
/// binary its boot is refused by name.
#[test]
fn hellos_binary_refuses_to_boot_an_entry() {
    link::set(Uses::NONE);
    let reply = boot_entry(&entry());
    assert!(reply.contains("Unlinked(\\\"widgets\\\")"), "{reply}");
}
