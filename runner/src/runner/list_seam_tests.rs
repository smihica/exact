//! The list seam's unlinked half (LLP 1047.001 D11): a plan with no list,
//! booted as an entry without lists boots it, answers every list entry as
//! the core does with no list, and as a binary with lists answers it but
//! where today's answers differ by what the binary links.
use super::list_seam::NoLists;
use super::*;
use crate::list_api::{
    CollectionFeedback, ListTextPolicy, ReorderBinding, ReorderGeometry, ReorderProgress,
    ReorderStep, ReorderToken,
};
use exact_kernel::Kernel;

const SOURCE: &str = r#"component App
  state n = 0
  action jump
    n = n + 1
    scrollIntoView("feed", "3")
  view
    column
      button "Go" testId="tap" press=jump
      list id="feed" testId="list"
        column testId="grip" height=20
          text `${n}`
"#;

/// Hello's composition on Apple: neither half of the list engine.
fn unlinked() -> (RunnerLinks, ModuleLinks<()>) {
    let seams = ModuleLinks {
        lists: &NoLists,
        ..ModuleLinks::ALL
    };
    (
        RunnerLinks {
            lists: None,
            ..RunnerLinks::ALL
        },
        seams,
    )
}

fn boot((links, seams): (RunnerLinks, ModuleLinks<()>)) -> Runner<()> {
    Runner::boot_with_delivery_linked(
        links,
        seams,
        contract::compile(SOURCE).unwrap(),
        (),
        Kernel::with_monospace(),
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        "/",
    )
    .unwrap()
}

fn key(r: &Runner<()>, test_id: &str) -> exact_kernel::NodeKey {
    r.kernel().find_first_by_test_id(test_id).unwrap()
}

fn view(r: &Runner<()>, test_id: &str) -> ViewId {
    r.kernel().node_by_key(key(r, test_id)).unwrap().id
}

fn feedback(view: ViewId) -> CollectionFeedback {
    CollectionFeedback {
        view,
        revision: 1,
        scroll_sequence: 1,
        offset: 0.,
        port_main: 100.,
        port_cross: 320.,
        cross: 320.,
        padding: [0.; 2],
        measurements: Vec::new(),
        focus_view: None,
        interaction_view: None,
    }
}

/// The journal's `scrollIntoView` refusals, without their times.
fn refusals(r: &Runner<()>) -> Vec<String> {
    r.journal()
        .filter_map(|line| line.find("scrollIntoView").map(|at| line[at..].to_owned()))
        .filter(|line| line.contains("refused"))
        .collect()
}

#[test]
fn a_plan_with_no_list_answers_every_list_entry_with_the_unlinked_half() {
    let mut r = boot(unlinked());
    assert!(!r.seams.lists.linked() && r.links.lists.is_none());
    let (list, grip) = (view(&r, "list"), key(&r, "grip"));
    let batch = r.batch;

    assert!(r.collections().is_empty() && r.collections_shallow().is_empty());
    assert_eq!(r.collection(list), None);
    let mut mounted = vec![(list, 7)];
    r.collection_mounted(list, &mut mounted);
    assert!(mounted.is_empty());
    assert!(r.collection_data().is_empty());
    assert_eq!(r.collections_bounded(8, 8, 8, 1024), Ok(Vec::new()));
    assert_eq!(r.collections_json(), "[]");
    assert_eq!(r.kept_positions_json(), "[]");
    r.collection_travel(list, 1, 1, 10.);

    assert_eq!(
        r.collection_snap(list, [0., 100., 0., 1000., f64::INFINITY]),
        None
    );
    assert!(r
        .collection_snap_areas(list, (0., [0., 100.], 4), [100., 0., 1000.])
        .is_empty());
    assert_eq!(
        r.collection_snap_stop(list, [0., 50.], [100., 0., 1000.]),
        None
    );
    assert_eq!(r.list_index(list, "3"), None);
    assert!(matches!(
        r.list_text(list, None, ListTextPolicy::Web),
        Err(RunnerError::Instance(InstanceError::List("unknown list")))
    ));

    assert!(r.collection_feedback_bytes(&[1, 2, 3]).is_err());
    let advanced = r.collection_feedback(feedback(list)).unwrap();
    assert!(advanced.receipts.is_empty() && advanced.error.is_none());
    assert!(r.collection_shown(list, 0.).unwrap().receipts.is_empty());
    assert!(r
        .collection_shown(list, f64::NAN)
        .unwrap()
        .receipts
        .is_empty());
    assert!(!r.set_list_covered(list, [10., 10.]));

    assert_eq!(r.reorder_binding(grip), None);
    let list_key = key(&r, "list");
    assert_eq!(r.reorder_geometry(list_key), None);
    let binding = ReorderBinding {
        handle: grip,
        list: list_key,
        wrapper: grip,
        root: grip,
        row_epoch: 1,
    };
    let geometry = ReorderGeometry {
        list: list_key,
        revision: 1,
        scroll_sequence: 1,
        scroll_top: 0.,
        port_width: 320.,
        port_height: 100.,
        row_width: 320.,
        total_extent: 20.,
    };
    assert!(r
        .begin_reorder(binding, geometry.clone())
        .unwrap()
        .is_none());
    assert!(r
        .begin_group_reorder(binding, geometry.clone(), true)
        .unwrap()
        .is_none());
    let token = ReorderToken {
        list: list_key,
        serial: 1,
    };
    assert!(!r.has_reorder(token));
    assert!(matches!(
        r.preview_reorder(token, geometry.clone(), 10.),
        Ok(ReorderProgress::Stale)
    ));
    assert!(matches!(
        r.preview_reorder_into(token, list_key, geometry.clone(), 10.),
        Ok(ReorderProgress::Stale)
    ));
    assert!(r.drop_reorder(token, geometry).unwrap().is_none());
    assert!(r.cancel_reorder(token).unwrap().is_none());
    assert!(r.reorder_frame(token).is_none());
    assert!(r.finish_reorder(token).unwrap().is_none());
    assert!(r
        .reorder_step(token, ReorderStep::Later)
        .unwrap()
        .is_empty());
    assert_eq!(r.reorder_json(), "null");

    assert_eq!(r.reveal_collection_index(list, 0).unwrap(), None);
    assert!(r
        .scroll_into_view_at(list, "3", "middle", "nearest")
        .is_err());
    r.scroll_into_view_at(list, "3", "start", "nearest")
        .unwrap();
    // Nothing committed: no direct entry above applies a batch unlinked.
    assert_eq!(r.batch, batch);
    let refused = format!("scrollIntoView #{list} refused: this artifact doesn't link lists");
    assert_eq!(refusals(&r), [refused]);
    assert!(r.into_view_json().contains("doesn't link lists"));

    // The command's request commits with the action and is refused there.
    r.dispatch(view(&r, "tap"), Event::Press).unwrap();
    assert!(refusals(&r).last().is_some_and(
        |line| line == "scrollIntoView feed refused: this artifact doesn't link lists"
    ));
}

#[test]
fn the_runner_half_runs_only_beside_the_instance_half() {
    let r = boot((RunnerLinks::ALL, ModuleLinks::ALL));
    assert!(r.seams.lists.linked() && r.links.lists.is_some());
    let seams = ModuleLinks {
        lists: &NoLists,
        ..ModuleLinks::ALL
    };
    let r = boot((RunnerLinks::ALL, seams));
    assert!(!r.seams.lists.linked() && r.links.lists.is_none());
    let links = RunnerLinks {
        lists: None,
        ..RunnerLinks::ALL
    };
    let r = boot((links, ModuleLinks::ALL));
    assert!(!r.seams.lists.linked() && r.links.lists.is_none());
}

/// The same plan and drive with and without lists linked: everything a host
/// or the agent sees agrees, but where it differs today. A direct
/// `scrollIntoView` and `reveal_collection_index` commit only linked, and a
/// refusal's reason is the engine's when linked.
#[test]
fn a_plan_with_no_list_is_driven_alike_with_and_without_lists_linked() {
    let drive = |seams| {
        let mut r = boot(seams);
        let list = view(&r, "list");
        r.collection_feedback(feedback(list)).unwrap();
        r.collection_shown(list, 0.).unwrap();
        let tap = view(&r, "tap");
        r.dispatch(tap, Event::Press).unwrap();
        let commanded = (r.batch, r.into_view_json(), refusals(&r));
        let before = r.batch;
        let direct = r
            .scroll_into_view_at(list, "3", "start", "nearest")
            .unwrap();
        let after_direct = r.batch;
        let revealed = r.reveal_collection_index(list, 0).unwrap();
        let after_reveal = r.batch;
        let shape = (
            r.collections_json(),
            r.kept_positions_json(),
            r.reorder_json(),
            r.collections().len(),
            r.kernel().live_count(),
        );
        (
            commanded,
            before,
            direct,
            after_direct,
            revealed,
            after_reveal,
            shape,
            list,
        )
    };
    let (linked, unlinked) = (
        drive((RunnerLinks::ALL, ModuleLinks::ALL)),
        drive(unlinked()),
    );
    let ((l_batch, l_json, l_refusals), l_before, _, l_after, l_revealed, l_reveal, l_shape, list) =
        linked;
    let (
        (u_batch, u_json, u_refusals),
        u_before,
        u_direct,
        u_after,
        u_revealed,
        u_reveal,
        u_shape,
        _,
    ) = unlinked;

    // The command commits either way; only the reason differs.
    assert_eq!(l_batch, u_batch);
    assert_eq!(
        l_refusals,
        ["scrollIntoView feed refused: no virtualized list has id feed"]
    );
    assert_eq!(
        u_refusals,
        ["scrollIntoView feed refused: this artifact doesn't link lists"]
    );
    assert!(l_json.contains("no virtualized list has id feed"));
    assert!(u_json.contains("this artifact doesn't link lists"));

    // A direct request and a reveal commit only where lists are linked.
    assert_eq!(l_before, u_before);
    assert!(l_after > l_before && l_reveal > l_after);
    assert_eq!((u_after, u_reveal), (u_before, u_before));
    assert_eq!(u_direct, CommitReceipt::default());
    assert_eq!((l_revealed, u_revealed), (None, None));
    let _ = list;

    assert_eq!(l_shape, u_shape);
}
