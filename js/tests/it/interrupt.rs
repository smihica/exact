//! Another thread stops a running call (LLP 1048.000 D10): the bytecode's
//! async break checks let Hermes stop a synchronous loop, and the call is
//! refused while the module goes on answering.

#![cfg(exact_js_engine)]

use exact_js::{Module, Placement};
use exact_plan::{Plan, Value};
use exact_runner::{Answer, DataError, DataSource, Dispatch, Reply, Store, Work};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::Duration;

const HBC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/spin.hbc"));
const APP: &str = "test.spin";

/// Also the render test's app: `words` answers at once, except for the name
/// "spin", for which it never returns on its own.
pub const SRC: &str = r#"
routes nav
  tab home "/" render=request
    page "/page/:name" render=request

component App
  resource words = words(params(nav, "name")) as shape string[]
  view
    column
      for w in words key = w
        text w
"#;

fn plan() -> Plan {
    contract::compile(SRC).expect("the fixture's Contract compiles")
}

fn unloaded() -> Module {
    let mut m = Module::new(HBC.to_vec(), APP, "");
    // Functional fixtures carry no wall-clock budget (castle.rs says why).
    m.set_budget_ms(f64::INFINITY);
    m.bind(&plan());
    m
}

fn module() -> Module {
    let mut m = unloaded();
    m.load().expect("the fixture loads");
    m
}

fn named(name: &str) -> [Value; 1] {
    [Value::list(vec![Value::str(name)])]
}

fn words(answer: Result<Answer, DataError>) -> Vec<String> {
    match answer {
        Ok(Answer::Now(Value::List(items))) => items
            .iter()
            .map(|w| w.as_str().unwrap_or("").to_string())
            .collect(),
        other => panic!("expected words, got {other:?}"),
    }
}

fn interrupted(result: &Result<Answer, DataError>, source: &str) -> bool {
    matches!(result, Err(DataError::Unavailable(m)) if *m == format!("`{source}` was interrupted"))
}

/// Told by a spinning call, through its native module, that it has started:
/// a channel for each test that spins, so none hears another's.
pub static SPINNING: [Mutex<Option<Sender<()>>>; 4] = [
    Mutex::new(None),
    Mutex::new(None),
    Mutex::new(None),
    Mutex::new(None),
];

/// The native module that tells `SPINNING[TEST]`.
pub struct Announcer<const TEST: usize>;
impl<const TEST: usize> exact_js::NativeModule for Announcer<TEST> {
    fn configure_storage(&mut self, _: exact_js::NativeStorage) -> Result<(), String> {
        Ok(())
    }
    fn call(&mut self, _: &serde_json::Value) -> Result<serde_json::Value, String> {
        if let Some(spinning) = SPINNING[TEST].lock().unwrap().as_ref() {
            let _ = spinning.send(());
        }
        Ok(serde_json::json!({}))
    }
}

#[test]
fn a_spinning_call_is_stopped_from_another_thread_and_the_module_answers_again() {
    let (spinning, started) = channel();
    *SPINNING[0].lock().unwrap() = Some(spinning);
    let root = std::env::temp_dir().join(format!("exact-interrupt-{}", std::process::id()));
    let mut m = unloaded().with_native(|_| Box::new(Announcer::<0>));
    m.configure_storage(root.join("data"), root.join("cache"), root.join("tmp"))
        .unwrap();
    m.activate().unwrap();
    let mut s = Store::new("", []);
    let stop = m.interrupt().expect("a module can be interrupted");
    // The other thread interrupts once the call is spinning, not before.
    let timer = std::thread::spawn(move || {
        let said = started.recv_timeout(Duration::from_secs(30));
        // Stops the call either way, so a call that never says so fails the test instead of hanging it
        stop.trigger();
        said.expect("the call never said it was spinning");
    });
    let spun = m.answer(&mut s, "words", &named("spin"));
    timer.join().unwrap();
    assert!(interrupted(&spun, "words"), "{spun:?}");
    // The interrupt was spent on that call: the module answers again.
    assert_eq!(
        words(m.answer(&mut s, "words", &named("calm"))),
        ["hello", "calm"]
    );
    assert_eq!(m.in_flight(), 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn console_formatting_does_not_swallow_an_interrupt() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../target")
        .join(format!("console-interrupt-{}", std::process::id()));
    let mut failures = Vec::new();
    for name in ["console-json", "console-stack", "console-string"] {
        let (spinning, started) = channel();
        *SPINNING[2].lock().unwrap() = Some(spinning);
        let mut m = unloaded().with_native(|_| Box::new(Announcer::<2>));
        m.configure_storage(root.join("data"), root.join("cache"), root.join("tmp"))
            .unwrap();
        m.activate().unwrap();
        let mut store = Store::new("", []);
        let stop = m.interrupt().unwrap();
        let interrupter = std::thread::spawn(move || {
            let said = started.recv_timeout(Duration::from_secs(30));
            stop.trigger();
            said.expect("the formatter never said it was spinning");
        });
        let result = m.answer(&mut store, "words", &named(name));
        interrupter.join().unwrap();
        if !interrupted(&result, "words") {
            failures.push(format!("{name}: {result:?}"));
        } else {
            assert!(crate::authored_logs(m.take_logs()).is_empty());
            assert_eq!(
                words(m.answer(&mut store, "words", &named("calm"))),
                ["hello", "calm"]
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_interrupt_with_nothing_running_stops_the_next_call_only() {
    let mut m = module();
    let mut s = Store::new("", []);
    m.interrupt().unwrap().trigger();
    let stopped = m.answer(&mut s, "words", &named("calm"));
    assert!(interrupted(&stopped, "words"), "{stopped:?}");
    assert_eq!(
        words(m.answer(&mut s, "words", &named("calm"))),
        ["hello", "calm"]
    );
}

#[test]
fn an_interrupt_before_the_module_loads_stops_its_initialization() {
    let mut m = unloaded();
    m.interrupt().unwrap().trigger();
    assert_eq!(
        m.load().unwrap_err(),
        "exact-js: the module was interrupted while it loaded"
    );
    assert!(!m.is_loaded());
    m.load().expect("the next load runs");
    assert_eq!(
        words(m.answer(&mut Store::new("", []), "words", &named("calm"))),
        ["hello", "calm"]
    );
}

#[test]
fn a_worker_instance_is_stopped_through_its_template() {
    let mut placed = unloaded().placed(Placement::Worker);
    placed.activate().unwrap();
    let stop = placed.interrupt().expect("the template's interrupt");
    let mut s = Store::new("", []);
    // One worker turn, its outcome sent back on a channel
    macro_rules! turn {
        ($name:expr) => {{
            let token = match placed.answer(&mut s, "words", &named($name)).unwrap() {
                Answer::Later(request) => request.continuation.expect("a turn"),
                Answer::Now(v) => panic!("a worker answers later, not {v:?}"),
            };
            let Dispatch::Run(Work::Later(work)) = placed.dispatch(token, &s) else {
                panic!("a turn for the owner");
            };
            let (tx, rx) = std::sync::mpsc::channel();
            work(Reply::new(move |outcome| {
                let _ = tx.send(outcome);
            }));
            rx
        }};
    }
    // A calm turn answered proves the worker's instance has loaded, so the
    // interrupt below meets the spin (or, if it comes first, the spin as the
    // next call) and never the load
    let calm = turn!("calm")
        .recv_timeout(Duration::from_secs(30))
        .expect("the owner answers a calm turn");
    let calm = placed.parse(&mut s, "words", &named("calm"), calm);
    assert_eq!(words(calm), ["hello", "calm"]);
    let spin = turn!("spin");
    stop.trigger();
    let outcome = spin
        .recv_timeout(Duration::from_secs(30))
        .expect("the owner's turn ends when interrupted");
    let refused = placed.parse(&mut s, "words", &named("spin"), outcome);
    assert!(interrupted(&refused, "words"), "{refused:?}");
}

#[test]
fn an_interrupted_first_turn_releases_its_fetch_and_finally() {
    for mode in ["sync", "drain"] {
        let root = super::storage::Root::new();
        let grants = "net.fetch https://outlive.test\nnet.fetch local-network development\nsecret.keep outlive.token\n";
        let (spinning, started) = channel();
        *SPINNING[3].lock().unwrap() = Some(spinning);
        let mut m = Module::new(
            include_bytes!(concat!(env!("OUT_DIR"), "/outlive.hbc")).to_vec(),
            "test.outlive",
            grants,
        )
        .with_native(|_| Box::new(Announcer::<3>));
        m.configure_storage(
            root.0.join("data"),
            root.0.join("cache"),
            root.0.join("tmp"),
        )
        .unwrap();
        m.set_budget_ms(f64::INFINITY);
        m.bind(&contract::compile("component App\n  resource r = interrupted(\"drain\") as shape string\n  resource cleanup = cleanup() as shape string\n  view\n    text r\n").unwrap());
        m.activate().unwrap();
        let stop = m.interrupt().unwrap();
        let timer = std::thread::spawn(move || {
            let said = started.recv_timeout(Duration::from_secs(30));
            stop.trigger();
            said.expect("the call never said it was spinning");
        });
        let result = m.answer(
            &mut Store::new(grants, []),
            "interrupted",
            &[Value::str(mode)],
        );
        timer.join().unwrap();
        assert!(interrupted(&result, "interrupted"), "{result:?}");
        assert_eq!(m.calls_open(), Some(0));
        assert_eq!(m.unsent(), 0);
        assert_eq!(
            m.query("cleanup", &[]).unwrap(),
            Value::str("false/true/Aborted")
        );
    }
}
