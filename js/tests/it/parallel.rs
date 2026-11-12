//! Two sends of a `parallel` mutation with equal arguments are two calls
//! (LLP 1161.000 D5): each is its own target at the seam, so the executor
//! parks both, and each reply resumes its own continuation, in any order.

#![cfg(exact_js_engine)]

use exact_js::Module;
use exact_kernel::{Kernel, PropId};
use exact_runner::{Outcome, Response, Runner};

const HBC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/castle.hbc"));
const APP: &str = "xyz.castle.test";
const GRANTS: &str = "net.fetch https://api.castle.xyz\nsecret.keep castle.session\n";

const SRC: &str = r#"
shape Session
  ok: bool
  username: string
  error: string

component App
  mutation session as shape Session parallel

  action twice
    send session = login("ada", "pw")
    send session = login("ada", "pw")

  view
    column
      match session
        case some(s)
          text s.username testId="user"
        case none
          text "none" testId="user"
"#;

fn boot() -> Runner<Module> {
    let mut module = Module::loaded(HBC.to_vec(), APP, GRANTS).unwrap();
    module.set_budget_ms(f64::INFINITY);
    Runner::boot(
        contract::compile(SRC).expect("the fixture's Contract compiles"),
        module,
        Kernel::with_monospace(),
        Default::default(),
        "/",
    )
    .unwrap()
}

fn user(r: &Runner<Module>) -> String {
    let k = r.kernel();
    let key = k.find_by_test_id("user")[0];
    k.node_by_key(key)
        .unwrap()
        .props
        .str(PropId::Text)
        .unwrap()
        .to_string()
}

fn login(username: &str) -> Outcome {
    let body = format!(r#"{{"data":{{"loginV2":{{"token":"t0k","username":"{username}"}}}}}}"#);
    Outcome::Response(Response {
        status: 200,
        headers: vec![("content-type".into(), "application/json".into())],
        body: body.into_bytes(),
    })
}

#[test]
fn equal_sends_answered_out_of_order_each_resume_their_own_call() {
    let mut r = boot();
    r.act("twice", vec![]).unwrap();
    let requests = r.take_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(r.data().in_flight(), 2, "both calls are parked");
    // The later send answers first: its reply is the newer outcome.
    r.fulfill(requests[1].ticket, login("later")).unwrap();
    assert_eq!(user(&r), "later");
    assert_eq!(r.data().in_flight(), 1);
    // The earlier's reply reaches its own call, whose continuation runs
    // (it keeps the session it was answered with); the slot stays the
    // later send's.
    r.fulfill(requests[0].ticket, login("earlier")).unwrap();
    assert_eq!(user(&r), "later");
    assert_eq!(r.data().in_flight(), 0);
    let kept = r.store().get("castle.session").unwrap_or_default();
    assert!(kept.contains("\"username\":\"earlier\""), "{kept}");
    assert!(r.pending().is_empty());
    assert!(!r.journal().any(|l| l.contains("not in flight")));
}
