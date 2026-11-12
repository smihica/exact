//! Calls through the source stacks used by generated hosts and renders.

use exact_data::{Mixed, Placed};
use exact_data_host::Storage;
use exact_logic::Swappable;
use exact_runner::{
    exact_canvas::{Context2d, DrawError, Frame},
    DataError, DataSource, Dispatch, Outcome, Placement, Request, Store, Value,
};

#[derive(Default)]
struct Leaf {
    changed: bool,
}

impl DataSource for Leaf {
    fn query(&mut self, _: &str, _: &[Value]) -> Result<Value, DataError> {
        Ok(Value::Unit)
    }

    fn note_storage_changed(&mut self) {
        self.changed = true;
    }

    fn take_storage_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    fn keeps_sent(&self, fetch: u64) -> bool {
        fetch == 7
    }

    fn host_continuation(&self, token: u64) -> bool {
        token == 7
    }

    fn module_delivery(
        &mut self,
        _: &mut Store,
        token: u64,
        _: Outcome,
    ) -> Result<Option<Request>, DataError> {
        Ok(Some(Request::continuation(token)))
    }

    fn draw_2d(
        &mut self,
        _: &str,
        _: &[Value],
        _: &Context2d,
        _: &Frame,
    ) -> Result<bool, DrawError> {
        Ok(true)
    }

    fn canvas_surfaces(&self) -> Vec<(String, usize)> {
        vec![("surface".into(), 0)]
    }
}

exact_logic::configured!(Configured, Leaf, || Swappable::off(Leaf::default()));

fn forwards(name: &str, mut source: impl DataSource) {
    source.note_storage_changed();
    let changed = source.take_storage_changed();
    let kept = source.keeps_sent(7) && !source.keeps_sent(8);
    let mut store = Store::new("", []);
    let delivered = source
        .module_delivery(&mut store, 7, Dispatch::again_outcome())
        .unwrap()
        .is_some();
    let drawn = source.draw_2d("surface", &[], &Context2d::new(), &Frame::default());
    assert_eq!(
        (changed, kept, delivered, drawn),
        (true, true, true, Ok(true)),
        "{name} forwards each call to its leaf"
    );
    assert!(!source.take_storage_changed());
}

#[test]
fn a_placed_source_forwards_leaf_calls() {
    forwards("Placed", Placed::new(Leaf::default(), Placement::Main));
}

#[test]
fn a_mixed_source_forwards_leaf_calls() {
    forwards("Mixed", Mixed::new(Leaf::default(), (), &[], &[]).unwrap());

    let mut mixed = Mixed::new((), Leaf::default(), &[], &[]).unwrap();
    assert!(!mixed.keeps_sent(7), "kept fetches belong to JavaScript");
    assert_eq!(
        mixed.draw_2d("surface", &[], &Context2d::new(), &Frame::default()),
        Ok(true),
        "a Rust canvas draws through its owner"
    );
}

#[test]
fn a_storage_source_forwards_leaf_calls() {
    forwards("Storage", Storage::new(Leaf::default()));
}

#[test]
fn a_swappable_source_forwards_leaf_calls() {
    forwards("Swappable", Swappable::off(Leaf::default()));
}

#[test]
fn a_configured_source_forwards_leaf_calls() {
    forwards("configured!", Configured::default());
}

#[cfg(unix)]
#[test]
fn a_render_source_forwards_leaf_calls() {
    let source = exact_render::Anonymous::new(Placed::new(Leaf::default(), Placement::Main));
    assert!(source.host_continuation(7));
    assert!(!source.host_continuation(8));
    forwards("Anonymous<Placed>", source);
}
