use super::*;
use std::cell::Cell;

thread_local! {
    pub(super) static DECODES: Cell<usize> = const { Cell::new(0) };
}

#[test]
fn unrelated_commits_do_not_decode_the_router_at_any_stack_depth() {
    let table = Table {
        routes: [
            ("home", "/", None, true),
            ("thread", "/t/:thread", Some(0), false),
        ]
        .into_iter()
        .map(|(name, pattern, parent, tab)| exact_route::Route {
            name: name.into(),
            pattern: pattern.into(),
            parent,
            tab,
            notfound: false,
        })
        .collect(),
    };
    let mut decodes = Vec::new();
    for depth in [1, 4, 16] {
        let mut runner = Runner::boot(
            test_fixture::plan(&table),
            (),
            exact_kernel::Kernel::with_monospace(),
            Default::default(),
            "/",
        )
        .unwrap();
        for visit in 1..=depth {
            runner
                .act("push", vec![Value::str(&format!("/t/{visit}"))])
                .unwrap();
        }
        runner.take_router_change();
        runner.act("back", vec![]).unwrap();
        let value = runner.slot("nav").unwrap().clone();
        assert_eq!(runner.derive("depth"), Some(&Value::Number(depth as f64)));
        let before = DECODES.get();
        for edit in 0..100 {
            runner
                .act("encode", vec![Value::str(&format!("edit {edit}"))])
                .unwrap();
        }
        decodes.push(DECODES.get() - before);
        assert_eq!(runner.slot("encoded"), Some(&Value::str("edit%2099")));
        assert!(crate::compare::same(runner.slot("nav").unwrap(), &value));
        assert_eq!(runner.location_ahead(), Some(format!("/t/{depth}")));
        let pending = runner.take_router_change().unwrap();
        assert_eq!(pending.visits.len(), depth);
        assert_eq!(pending.removed.len(), 1);
        runner
            .act("encode", vec![Value::str("after drain")])
            .unwrap();
        assert!(runner.take_router_change().is_none());
    }
    assert_eq!(
        decodes,
        [0, 0, 0],
        "100 unrelated commits at depths 1, 4 and 16"
    );
}
