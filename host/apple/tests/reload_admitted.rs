//! LLP 1047.001 D6: a development binary links every family, so a reload
//! from hello's plan into a plan that uses one is committed as the next
//! generation. `reload_refused.rs` is hello's composition.

use exact_apple::abi::{Bridge, Hooks};

#[path = "reload/common.rs"]
mod common;
use common::{boot, digest, media, plan, source, NoData, ACTIVITIES, ANIMATED, TRACING};

#[test]
fn an_all_linked_binary_reloads_from_hello_into_each_family() {
    for (view, next) in [
        ("filter", plan("text \"a\" filter=\"blur(2px)\"")),
        (
            "svg",
            plan(
                "svg viewBox=\"0 0 10 10\"
      circle cx=5 cy=5 r=4",
            ),
        ),
        ("hatch", plan("view hatch=\"avatar\"")),
        ("checkbox", plan("input type=\"checkbox\"")),
        ("animation", source(ANIMATED)),
        (
            "gradient",
            plan("column background-image=\"linear-gradient(red, blue)\""),
        ),
        (
            "tablist",
            plan(
                "row role=\"tablist\"
      button \"a\" role=\"tab\"",
            ),
        ),
        (
            "flight",
            plan(
                "column sharedElement=\"hero\"
      text \"a\"",
            ),
        ),
        (
            "sticky",
            plan(
                "column position=\"sticky\" top=0
      text \"a\"",
            ),
        ),
        ("activities", source(ACTIVITIES)),
        ("file input", plan("input type=\"file\" id=\"f\"")),
        ("trace", source(TRACING)),
        ("wake lock", plan("text \"a\" wakelock=\"screen\"")),
        (
            "route",
            plan("column navigationKey=\"a\"\n      text \"a\""),
        ),
        (
            "sheet route",
            plan("column navigationKey=\"a\"\n      column navigationKey=\"b\" navigationPresentation=\"modal\"\n        text \"a\""),
        ),
        (
            "bar",
            plan("column navigationKey=\"a\"\n      column navigationKey=\"a\" navigationBar=\"none\"\n        text \"a\""),
        ),
        (
            "history",
            plan("column navigationKey=\"a\" navigationHistory=\"back\"\n      text \"a\""),
        ),
    ]
    .into_iter()
    .chain(media().map(|(next, name)| (name, next)))
    {
        let mut bridge = Bridge::<NoData>::new();
        boot(&mut bridge, &plan("text \"Hello world\""));
        let before = digest(&mut bridge);
        let len = bridge.input_write(&next);
        bridge.prepare_plan(len, NoData, Hooks::none(), 390.0, 844.0);
        let out = bridge.commit_plan();
        let reply = String::from_utf8_lossy(bridge.output_bytes(out as usize)).into_owned();
        assert!(!reply.contains("Unlinked"), "{view}: {reply}");
        assert_ne!(digest(&mut bridge), before, "{view}");
    }
}
