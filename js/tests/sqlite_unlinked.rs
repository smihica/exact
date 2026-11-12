//! A separate binary keeps the process-wide SQLite provider slot empty.
#![cfg(exact_js_engine)]

use exact_js::Module;
use exact_plan::Value;
use exact_runner::{Answer, DataSource, Dispatch, Store, Work};

#[test]
fn javascript_sqlite_open_with_a_grant_but_no_provider_names_the_missing_link() {
    assert!(ibex2::stdlib::sqlite::linked().is_none());
    let grants = "fs.read app:/data\nfs.write app:/data\nfs.write app:/cache/widget-hidden\nfs.write app:/cache/widget-copy\nfs.read doc:/\nfs.write doc:/\ndoc.keep\nsqlite.open app:/data/notes.db\nnet.fetch https://example.test\nsecret.keep session\nfs.read bundle:/Resources/shipped\n";
    let plan = contract::compile(
        r#"
shape Result
  text: string
component App
  mutation output as shape Result
  action invoke(op: string, value: string)
    send output = work(op, value)
  view
    text "storage"
"#,
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!("exact-sqlite-unlinked-{}", std::process::id()));
    let mut module = Module::new(
        include_bytes!(concat!(env!("OUT_DIR"), "/storage.hbc")).to_vec(),
        "dev.exact.storage-test",
        grants,
    );
    module.set_budget_ms(f64::INFINITY);
    module
        .configure_storage(root.join("data"), root.join("cache"), root.join("tmp"))
        .unwrap();
    module.bind(&plan);
    module.activate().unwrap();
    let mut store = Store::new(grants, Vec::<(String, Vec<u8>)>::new());
    let args = [Value::str("hold"), Value::str("")];
    let mut answer = module.answer(&mut store, "work", &args).unwrap();
    let result = loop {
        match answer {
            Answer::Now(value) => break value,
            Answer::Later(request) => {
                let token = request.continuation.expect("SQLite runs as storage work");
                let Dispatch::Run(Work::Now(work)) = module.dispatch(token, &store) else {
                    panic!("the only storage request must run");
                };
                let outcome = std::thread::spawn(work).join().unwrap();
                answer = module.parse(&mut store, "work", &args, outcome).unwrap();
            }
        }
    };
    module.unload();
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        result,
        Value::record(vec![Value::str("Unlinked(\"sqlite\")")])
    );
    assert!(ibex2::stdlib::sqlite::linked().is_none());
}
