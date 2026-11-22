//! Canvas 2D's link (LLP 1047.001 §9.21, `TurnLinks::canvas`): a plan with
//! a canvas links it and draws as an archive that links everything does; a
//! plan without one links none of it, and a source's capture there is
//! refused by the link's name.
use super::*;
use crate::link::{Capability, Links, TurnLinks};
use exact_kernel::MonospaceMeasurer;
use exact_plan::Value;
use exact_runner::exact_canvas::{Context2d, DrawError, Frame};
use exact_runner::{Answer, DataError, Event, Request, Store};

#[derive(Clone, Default)]
struct Ink;
impl DataSource for Ink {
    fn query(&mut self, source: &str, _: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(source.into()))
    }
    fn canvas_surfaces(&self) -> Vec<(String, usize)> {
        vec![("dot".into(), 0)]
    }
    fn draw_2d(
        &mut self,
        _: &str,
        _: &[Value],
        ctx: &Context2d,
        _: &Frame,
    ) -> Result<bool, DrawError> {
        ctx.fill_rect(0., 0., 4., 4.);
        Ok(true)
    }
    fn answer(&mut self, _: &mut Store, _: &str, _: &[Value]) -> Result<Answer, DataError> {
        Ok(Answer::Later(Request::capture_canvas(
            "dot",
            "app:/tmp/dot.png",
        )))
    }
}

const CANVAS: &str = r#"component A
  state on = false
  action go
    on = not on
  view
    column
      button press=go testId="go"
        text "Go"
      canvas surface=dot() width=(on ? 60 : 40) height=40
"#;

const CAPTURE: &str = r#"component A
  resource image = capture() as shape string
  view
    text image
"#;

fn uses(source: &str) -> crate::link::Uses {
    exact_runner::uses::apple_uses(&contract::compile(source).unwrap())
}

fn boot(source: &str, turn: TurnLinks<Ink>) -> (Host<Ink>, String) {
    let plan = contract::compile(source).unwrap().encode();
    let (mut host, first) = Host::boot_stored_after_decode(
        PlanBytes::Copied(&plan),
        Ink,
        Box::new(MonospaceMeasurer::default()),
        scale::LaunchLayout::NONE,
        exact_runner::Viewport::sized(390.0, 844.0),
        None,
        Vec::new(),
        None,
        None,
        None,
        None,
        "/",
        false,
        None,
        Links { turn, ..Links::ALL },
        None,
        |_| Ok(()),
    )
    .unwrap();
    host.commit_boot();
    (host, first)
}

/// A batch without what two runs in one process differ in alone: each
/// canvas list's address (the replay reads it out of this process's memory)
/// and a canvas's lifetime, which counts mounts across the process.
fn comparable(batch: String) -> String {
    let mut batch: serde_json::Value = serde_json::from_str(&batch).unwrap();
    for op in batch["ops"].as_array_mut().unwrap() {
        if let Some(lifetime) = op.get_mut("lifetime") {
            *lifetime = 0.into();
        }
        if let Some(lists) = op.get_mut("lists").and_then(|l| l.as_array_mut()) {
            for list in lists {
                list[0] = 0.into();
            }
        }
    }
    batch.to_string()
}

/// The boot's batch, two presses and the frames after each.
fn run(source: &str, turn: TurnLinks<Ink>) -> Vec<String> {
    let (mut host, first) = boot(source, turn);
    let go = {
        let k = host.runner.kernel();
        k.node_by_key(k.find_by_test_id("go")[0]).unwrap().id
    };
    let mut batches = vec![first];
    for at in [100.0, 2_000.0] {
        batches.push(host.dispatch_at(go, Event::Press, at));
        for dt in [16.0, 300.0] {
            batches.push(host.tick(at + dt));
        }
    }
    batches.into_iter().map(comparable).collect()
}

#[test]
fn a_canvas_links_with_surfaces_and_draws_as_everything_linked() {
    let used = uses(CANVAS);
    assert!(used.has(Capability::Surfaces));
    assert!(TurnLinks::<Ink>::of(used).canvas.is_some());
    let linked = run(CANVAS, TurnLinks::of(used));
    assert_eq!(linked, run(CANVAS, TurnLinks::of(crate::link::ALL)));
    assert!(linked.iter().any(|b| b.contains("\"op\":\"canvas2d\"")));
    // The link is what draws it.
    let unlinked = run(CANVAS, TurnLinks::of(crate::link::Uses::NONE));
    assert!(!unlinked.iter().any(|b| b.contains("\"op\":\"canvas2d\"")));
}

#[test]
fn without_a_canvas_none_is_linked_and_a_capture_is_refused_by_name() {
    assert!(TurnLinks::<Ink>::of(uses(CAPTURE)).canvas.is_none());
    let (mut host, _) = boot(CAPTURE, TurnLinks::of(uses(CAPTURE)));
    host.activate_data();
    let requests = host.take_requests();
    assert_eq!(requests.len(), 1);
    let mut presenter = crate::batch::Batch::new();
    host.surface_work(&mut presenter, &requests[0], None);
    let (ticket, outcome) = host.take_request_refusal(true).expect("refused at once");
    assert_eq!(ticket, requests[0].ticket);
    assert!(
        format!("{outcome:?}").contains("Unlinked(\\\"surfaces\\\")"),
        "{outcome:?}"
    );
}
