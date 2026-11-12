//! `resource … transient` (LLP 1156.005 D4).

/// `transient` after a resource's shape reaches its plan row (LLP 1156.005 D4).
#[test]
fn a_transient_resource_is_marked_in_the_plan() {
    let src = include_str!("../../../corpus/surface.contract");
    let line = src
        .lines()
        .find(|l| l.trim_start().starts_with("resource "))
        .expect("a resource");
    let plan = contract::compile(src).unwrap();
    assert!(!plan.resources[0].transient);
    let marked = src.replace(line, &format!("{line} transient"));
    assert!(contract::compile(&marked).unwrap().resources[0].transient);
}
