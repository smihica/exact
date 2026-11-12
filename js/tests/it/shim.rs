//! Console diagnostics through the native Hermes bridge.
#![cfg(exact_js_engine)]

use exact_js::Module;
use exact_runner::DataSource;

fn console(source: &str) -> (String, Vec<String>) {
    let plan = contract::compile(&format!(
        "component App\n  resource result = {source}() as shape string\n  view\n    text result\n"
    ))
    .unwrap();
    let mut module = Module::new(
        include_bytes!(concat!(env!("OUT_DIR"), "/pure.hbc")).to_vec(),
        "test.pure",
        "",
    );
    module.set_budget_ms(f64::INFINITY);
    module.bind(&plan);
    module.activate().unwrap();
    let mut store = exact_runner::Store::new("", vec![]);
    let value = super::castle::now(module.answer(&mut store, source, &[]).unwrap());
    let logs = crate::authored_logs(module.take_logs());
    assert!(module.take_logs().is_empty(), "logs are taken once");
    (value.as_str().unwrap().to_owned(), logs)
}

#[test]
fn console_reports_throwing_abort_listeners() {
    let (answer, logs) = console("consoleAbortListeners");
    assert_eq!(answer, "reached the return");
    assert_eq!(
        logs,
        [
            "Uncaught Error: boom",
            "Uncaught RangeError: listener",
            "later listener",
        ]
    );
}

#[test]
fn console_trailing_newline_does_not_add_an_empty_line() {
    let (answer, logs) = console("consoleTrailingNewline");
    assert_eq!(answer, "reached the return");
    assert_eq!(logs, ["done"]);
}

#[test]
fn console_keeps_a_trailing_carriage_return() {
    let (answer, logs) = console("consoleTrailingCarriageReturn");
    assert_eq!(answer, "reached the return");
    assert_eq!(logs, ["100%\r"]);
}

#[test]
fn console_splits_lines_within_each_record() {
    let (answer, logs) = console("consoleLineEndings");
    assert_eq!(answer, "reached the return");
    assert_eq!(
        logs,
        [
            "done",
            "100%\r",
            "next",
            "",
            "",
            "a",
            "b",
            "x\0é😀",
            "end",
            ""
        ]
    );
}

#[test]
fn console_errors_keep_their_name_message_and_stack() {
    let (stack, logs) = console("consoleErrors");
    assert!(stack.starts_with("TypeError: boom\n"), "{stack}");
    assert!(stack.lines().count() > 1, "the error has a stack");
    let expected = format!("{stack}\nload failed {stack}");
    assert_eq!(logs, expected.lines().collect::<Vec<_>>());
}

#[test]
fn console_cycles_do_not_fail_the_answer() {
    let (answer, logs) = console("consoleCycle");
    assert_eq!(answer, "reached the return");
    assert_eq!(logs, ["row [object Object]", "after"]);
}

#[test]
fn console_bigints_do_not_fail_the_answer() {
    let (answer, logs) = console("consoleBigInt");
    assert_eq!(answer, "reached the return");
    assert_eq!(logs, ["bigint 1 [object Object]"]);
}

#[test]
fn console_falls_back_even_when_application_formatters_throw() {
    let (answer, logs) = console("consoleFallback");
    assert_eq!(answer, "reached the return");
    assert_eq!(
        logs,
        [
            "RangeError: no stack",
            "fallback",
            "[unprintable] after",
            "null undefined true 42 Symbol(label) {\"ok\":true}",
        ]
    );
}

#[test]
fn console_keeps_empty_lines() {
    let (answer, logs) = console("consoleEmpty");
    assert_eq!(answer, "reached the return");
    assert_eq!(logs, [""]);
    let (_, logs) = console("consoleEmptyBatch");
    assert_eq!(logs, ["", "middle", "", ""]);
}
