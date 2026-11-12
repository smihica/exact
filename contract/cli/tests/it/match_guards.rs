fn app(body: &str, member: bool) -> String {
    let (key, option) = if member {
        ("Key(value=\"1\")", "some(Key(value=\"2\"))")
    } else {
        ("\"1\"", "some(\"2\")")
    };
    let body: String = body.lines().map(|line| format!("    {line}\n")).collect();
    format!(
        "shape Key\n  value: string\nshape R\n  ok: bool\ncomponent App\n  state k = {key}\n  state o = {option}\n  state n = 0\n  state seen = 0\n  mutation m as shape R\n  action writeCount\n    n = 1\n  action go\n{body}  view\n    button \"go\" press=go\n"
    )
}

const PATHS: &[(&str, &str)] = &[
    (
        "enter",
        "if k == \"1\"\n  FIRST\nmatch o\n  case some(v)\n    if v == \"2\"\n      SECOND\n  case none\n    seen = 0",
    ),
    (
        "leave",
        "match o\n  case some(v)\n    if v == \"2\"\n      FIRST\n  case none\n    seen = 0\nif k == \"1\"\n  SECOND",
    ),
    (
        "enclosing guard",
        "if k == \"1\"\n  match o\n    case some(v)\n      if v == \"2\"\n        FIRST\n      if v == \"2\"\n        SECOND\n    case none\n      seen = 0",
    ),
    (
        "nested binders",
        "match o\n  case some(v)\n    if v == \"2\"\n      FIRST\n    match some(\"1\")\n      case some(w)\n        if w == \"1\"\n          SECOND\n      case none\n        seen = 0\n  case none\n    seen = 0",
    ),
    (
        "sibling binders",
        "match o\n  case some(v)\n    if v == \"2\"\n      FIRST\n  case none\n    seen = 0\nmatch some(\"1\")\n  case some(w)\n    if w == \"1\"\n      SECOND\n  case none\n    seen = 0",
    ),
];

fn detects(first: &str, second: &str, diagnostic: &str, called: bool) {
    for (case, body) in PATHS {
        for member in [false, true] {
            let body = if member {
                body.replace("k ==", "k.value ==")
                    .replace("v ==", "v.value ==")
                    .replace("w ==", "w.value ==")
                    .replace("some(\"1\")", "some(Key(value=\"1\"))")
            } else {
                body.to_string()
            };
            for shadow in [false, true] {
                let body = if shadow {
                    body.replace("some(v)", "some(k)")
                        .replace("some(w)", "some(k)")
                        .replace("if v", "if k")
                        .replace("if w", "if k")
                } else {
                    body.clone()
                };
                let src = app(&body, member);
                let line = |text| src.lines().position(|l| l.contains(text)).unwrap() as u32 + 1;
                let first_line = line(if called { "n = 1" } else { "FIRST" });
                let second_line = line("SECOND");
                let src = src.replace("FIRST", first).replace("SECOND", second);
                let (result, warnings) = contract::compile_warned(&src);
                let diagnostics = if diagnostic == "analyze-send-twice" {
                    let error = result
                        .err()
                        .unwrap_or_else(|| panic!("two sends compiled:\n{src}"));
                    assert_eq!(error.related[0].span.line, first_line, "{src}");
                    vec![error]
                } else {
                    result.unwrap_or_else(|e| panic!("{src}\n{e}"));
                    assert!(warnings.iter().all(|w| w.is_warning()));
                    for warning in &warnings {
                        assert!(
                            warning.message.contains(&format!("line {first_line}")),
                            "{warning}"
                        );
                    }
                    warnings
                };
                let got: Vec<_> = diagnostics
                    .iter()
                    .map(|e| (e.id.as_str(), e.span.line))
                    .collect();
                assert_eq!(
                    got,
                    [(diagnostic, second_line)],
                    "{case}, member={member}, shadow={shadow}\n{src}"
                );
            }
        }
    }
}

#[test]
fn shadowing_match_does_not_hide_a_second_send() {
    detects(
        "send m = save(\"a\")",
        "send m = save(\"b\")",
        "analyze-send-twice",
        false,
    );
}

#[test]
fn shadowing_match_does_not_hide_a_read_after_assignment() {
    detects("n = n + 1", "seen = n", "analyze-read-after-assign", false);
}

#[test]
fn shadowing_match_does_not_hide_a_read_after_a_call_assigned() {
    detects(
        "writeCount()",
        "seen = n",
        "analyze-read-after-assign",
        true,
    );
}

#[test]
fn shadowing_match_does_not_hide_a_send_and_assignment() {
    for (first, second) in [
        ("send m = save(\"a\")", "m = none"),
        ("m = none", "send m = save(\"a\")"),
    ] {
        detects(first, second, "analyze-send-assigned", false);
    }
}

#[test]
fn match_scopes_keep_exclusive_paths_separate() {
    for body in [
        "match o\n  case some(k)\n    if k == \"1\"\n      FIRST\n    if k == \"2\"\n      SECOND\n  case none\n    seen = 0",
        "if k == \"1\"\n  FIRST\nmatch o\n  case some(k)\n    seen = 0\n  case none\n    if k == \"2\"\n      SECOND",
        "if seen == 1\n  FIRST\nmatch o\n  case some(k)\n    if seen == 2\n      SECOND\n  case none\n    seen = 0",
        "match o\n  case some(k)\n    FIRST\n  case none\n    SECOND",
    ] {
        for (first, second) in [
            ("send m = save(\"a\")", "send m = save(\"b\")"),
            ("n = n + 1", "seen = n"),
            ("writeCount()", "seen = n"),
            ("send m = save(\"a\")", "m = none"),
            ("m = none", "send m = save(\"a\")"),
        ] {
            let src = app(&body.replace("FIRST", first).replace("SECOND", second), false);
            let (result, warnings) = contract::compile_warned(&src);
            result.unwrap_or_else(|e| panic!("{src}\n{e}"));
            assert!(warnings.is_empty(), "{src}\n{warnings:?}");
        }
    }
}

fn preserves_outer_guards(first: &str, second: &str) {
    for body in [
        "if k == \"1\"\n  FIRST\nmatch o\n  case some(v)\n    seen = 0\n  case none\n    seen = 0\nif k == \"2\"\n  SECOND",
        "if k == \"1\"\n  match o\n    case some(v)\n      seen = 0\n    case none\n      seen = 0\n  FIRST\nif k == \"2\"\n  SECOND",
        "if k == \"1\"\n  match o\n    case some(v)\n      FIRST\n    case none\n      seen = 0\nif k == \"2\"\n  SECOND",
        "match o\n  case some(k)\n    if k == \"1\"\n      FIRST\n    match o\n      case some(v)\n        seen = 0\n      case none\n        seen = 0\n    if k == \"2\"\n      SECOND\n  case none\n    seen = 0",
    ] {
        for member in [false, true] {
            let body = if member {
                body.replace("k ==", "k.value ==")
            } else {
                body.to_string()
            };
            for binder in ["v", "k"] {
                let body = body.replace("some(v)", &format!("some({binder})"));
                let src = app(&body.replace("FIRST", first).replace("SECOND", second), member);
                let (result, warnings) = contract::compile_warned(&src);
                result.unwrap_or_else(|e| panic!("{src}\n{e}"));
                assert!(warnings.is_empty(), "{src}\n{warnings:?}");
            }
        }
    }
}

#[test]
fn shadowing_match_preserves_exclusive_send_guards() {
    preserves_outer_guards("send m = save(\"a\")", "send m = save(\"b\")");
}

#[test]
fn shadowing_match_preserves_exclusive_read_guards() {
    preserves_outer_guards("n = n + 1", "seen = n");
    preserves_outer_guards("writeCount()", "seen = n");
}

#[test]
fn shadowing_match_preserves_exclusive_send_and_assignment_guards() {
    preserves_outer_guards("send m = save(\"a\")", "m = none");
    preserves_outer_guards("m = none", "send m = save(\"a\")");
}

#[test]
fn assignments_in_a_match_invalidate_slot_facts_but_keep_binder_facts() {
    for (first, second, diagnostic) in [
        (
            "send m = save(\"a\")",
            "send m = save(\"b\")",
            "analyze-send-twice",
        ),
        ("send m = save(\"a\")", "m = none", "analyze-send-assigned"),
        ("m = none", "send m = save(\"a\")", "analyze-send-assigned"),
    ] {
        for member in [false, true] {
            for (body, invalidated) in [
                ("if k == \"1\"\n  FIRST\nmatch o\n  case some(k)\n    k = NEXT\n  case none\n    seen = 0\nif k == \"2\"\n  SECOND", true),
                ("match o\n  case some(k)\n    if k == \"1\"\n      FIRST\n    k = NEXT\n    if k == \"2\"\n      SECOND\n  case none\n    seen = 0", false),
            ] {
                let body = if member {
                    body.replace("k ==", "k.value ==").replace("NEXT", "Key(value=\"2\")")
                } else {
                    body.replace("NEXT", "\"2\"")
                };
                let src = app(&body.replace("FIRST", first).replace("SECOND", second), member);
                let (result, warnings) = contract::compile_warned(&src);
                if invalidated {
                    if diagnostic == "analyze-send-twice" {
                        assert_eq!(result.unwrap_err().id, diagnostic, "{src}");
                    } else {
                        result.unwrap_or_else(|e| panic!("{src}\n{e}"));
                        assert!(warnings.iter().any(|w| w.id == diagnostic), "{src}\n{warnings:?}");
                    }
                } else {
                    result.unwrap_or_else(|e| panic!("{src}\n{e}"));
                    assert!(warnings.is_empty(), "{src}\n{warnings:?}");
                }
            }
        }
    }
}
