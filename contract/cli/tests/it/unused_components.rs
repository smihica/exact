//! A component nothing uses has its literal values checked (LLP 1156.005 D3):
//! lowering walks from the root, and a bad value there was refused only at
//! the component's first use.

#[test]
fn a_bad_literal_in_a_component_nothing_uses_is_refused() {
    let e = contract::compile(
        "component App\n  view\n    text \"Hi\"\n\ncomponent Badge\n  view\n    text \"New\" color=\"notacolor\"\n",
    )
    .expect_err("refused");
    assert_eq!(e.id, "lower-attr-value", "{}", e.message);
    assert_eq!(e.span.line, 7);
}

#[test]
fn what_needs_a_caller_is_left_to_the_caller() {
    // A value read from a prop is the caller's to give; an unused component
    // may also be one nothing has used yet in a chain of unused ones.
    let src = "component App\n  view\n    text \"Hi\"\n\ncomponent Badge\n  props\n    tint: string\n  view\n    Dot(tint=tint)\n\ncomponent Dot\n  props\n    tint: string\n  view\n    view width=8 height=8 background-color=tint\n";
    contract::compile(src).expect("builds");
    let chained = src.replace("background-color=tint", "background-color=\"nope\"");
    let e = contract::compile(&chained).expect_err("refused");
    assert_eq!(e.id, "lower-attr-value", "{}", e.message);
}

#[test]
fn a_used_component_is_refused_once() {
    let src = "component App\n  view\n    Badge()\n\ncomponent Badge\n  view\n    text \"New\" color=\"notacolor\"\n";
    let e = contract::compile(src).expect_err("refused");
    assert_eq!(e.id, "lower-attr-value", "{}", e.message);
}
