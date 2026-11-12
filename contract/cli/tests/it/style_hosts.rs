//! @ref LLP 1159.000 D4 — `style-undrawn`: for each target the app ships,
//! each style row it sets that the target's host does not draw there (the
//! schema's `hosts` column), once a row, scope and reason, naming each
//! target; `best-effort` says nothing; a value that leaves the row unset
//! is not judged.

use std::path::PathBuf;

fn app(name: &str, targets: &str, view: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("exact2-style-hosts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("app.json"),
        format!("{{\"app\": {{\"id\": \"dev.test.styles\", \"name\": \"Styles\"}}, \"host\": {{{targets}}}}}"),
    )
    .unwrap();
    std::fs::write(dir.join("app.contract"), view).unwrap();
    dir.join("app.contract")
}

/// The `style-undrawn` messages, in source order.
fn undrawn(path: &std::path::Path) -> Vec<String> {
    let (plan, warnings) = contract::compile_path_all_warned(path, false);
    plan.unwrap_or_else(|e| panic!("{e:?}"));
    warnings
        .iter()
        .filter(|w| w.id == "style-undrawn")
        .map(|w| w.message.clone())
        .collect()
}

/// Two blended shapes, one by its own row and one by a class, and a
/// filtered box.
const BLENDED: &str = "style Multiplied\n  mix-blend-mode=\"multiply\"\n\ncomponent App\n  view\n    column\n      box filter=\"blur(4px)\" width=20 height=20\n      svg width=20 height=20\n        rect width=10 height=10 mix-blend-mode=\"screen\"\n        rect width=10 height=10 class=Multiplied\n";

#[test]
fn a_blend_is_undrawn_on_ios_and_on_tvos_which_reads_its_column() {
    let said = undrawn(&app(
        "blend",
        "\"ios\": {}, \"macos\": {}, \"web\": {}",
        BLENDED,
    ));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].starts_with("`mix-blend-mode` is not drawn on ios (app.json `host.ios`): UIKit has no public blend on a layer"),
        "{said:?}"
    );
    // The shape's own row and its class's.
    assert!(
        said[0].contains("set 2 times, the first at line 9"),
        "{said:?}"
    );
    let tv = undrawn(&app("tv", "\"tvos\": {}", BLENDED));
    assert_eq!(tv.len(), 1, "{tv:?}");
    assert!(
        tv[0].starts_with("`mix-blend-mode` is not drawn on tvos"),
        "{tv:?}"
    );
}

#[test]
fn each_host_says_its_own_reason_and_a_host_that_draws_it_says_nothing() {
    // The Mac, Linux and Windows blend; the web is the oracle.
    assert_eq!(
        undrawn(&app(
            "drawn",
            "\"macos\": {}, \"linux\": {}, \"windows\": {}, \"web\": {}",
            BLENDED
        )),
        Vec::<String>::new()
    );
    // iOS and Android each have a reason of their own: a line each.
    let said = undrawn(&app("both", "\"ios\": {}, \"android\": {}", BLENDED));
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said.iter()
            .any(|m| m.contains("on ios (app.json `host.ios`): UIKit")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|m| m
            .contains("on android (app.json `host.android`): the reader's stream has no blend")),
        "{said:?}"
    );
}

#[test]
fn best_effort_and_an_unset_value_say_nothing() {
    // A backdrop blur differs on iOS and macOS, and a turned box's shadow
    // where the paint walk draws: `vocab` says both, the lint neither.
    let glass = "component App\n  view\n    column\n      box backdrop-filter=\"blur(8px)\" filter=\"drop-shadow(2px 2px 2px #000000)\" width=20 height=20\n      svg width=20 height=20\n        rect width=10 height=10 mix-blend-mode=\"normal\"\n";
    assert_eq!(
        undrawn(&app(
            "glass",
            "\"ios\": {}, \"macos\": {}, \"linux\": {}, \"android\": {}",
            glass
        )),
        Vec::<String>::new()
    );
}

#[test]
fn it_is_information_like_tamber_unreachable() {
    assert!(contract_lower::warning_rank::information("style-undrawn"));
}
