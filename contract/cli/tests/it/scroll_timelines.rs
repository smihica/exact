//! Scroll timelines in Contract (LLP 1057.003 D5): `scroll-timeline` on a
//! scroller and `animation-timeline` naming it, or `scroll()`, reach the
//! kernel, resolve through `timeline-scope`, and are placed by layout over
//! the scroller's scroll range, so an engine given the scroller's offset
//! holds the consumer there with nothing in between: the follower moves in
//! the frame the scroller does (#114).

use crate::source::{boxed, Source};
use exact_kernel::timeline::{Found, TimelineScroller};
use exact_kernel::{motion_node, Kernel, MotionSync, Offer};
use exact_motion::{Engine, NamedTimeline, Property};
use exact_plan::{Plan, Value};
use exact_runner::{DataError, DataSource, Runner};

pub(crate) struct NoData;
impl DataSource for NoData {
    fn query(&mut self, source: &str, _: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(source.into()))
    }
}

/// An app booted, laid out in a `width` × `height` viewport, and an engine
/// that has heard every node and the placement, as a native host's boot
/// gives it.
pub(crate) struct Laid {
    pub(crate) runner: Runner<Source>,
    pub(crate) engine: Engine,
}

impl Laid {
    pub(crate) fn boot(app: &str, width: f32, height: f32) -> Self {
        let plan = contract::compile(app).unwrap_or_else(|e| panic!("{e}"));
        let mut runner = Runner::boot(
            Plan::decode(&plan.encode()).unwrap(),
            boxed(NoData),
            Kernel::with_monospace(),
            Default::default(),
            "/",
        )
        .unwrap();
        let k = runner.kernel_mut();
        let env = exact_kernel::style::Env {
            viewport_width: width,
            viewport_height: height,
            ..k.env()
        };
        k.set_env(env).unwrap();
        let mut laid = Self {
            runner,
            engine: Engine::new(),
        };
        laid.layout(width, height);
        let k = laid.runner.kernel();
        let mut sync = MotionSync::default();
        let keys: Vec<_> = (k.arena().iter_live()).map(|s| k.arena().key(s)).collect();
        for key in keys {
            k.motion_sync_node(key, &mut sync);
        }
        sync.apply(&mut laid.engine).unwrap();
        laid.place();
        laid
    }

    pub(crate) fn layout(&mut self, width: f32, height: f32) {
        let k = self.runner.kernel_mut();
        let root = k.roots()[0];
        k.compute_layout(root, Offer::definite(width, height))
            .unwrap();
    }

    /// Place the timelines after a layout, as a host does.
    pub(crate) fn place(&mut self) {
        for (node, binding) in self.runner.kernel_mut().place_timelines() {
            self.engine.set_animation_timeline(node, binding);
        }
    }

    pub(crate) fn node(&self, test_id: &str) -> u64 {
        motion_node(self.runner.kernel().find_by_test_id(test_id)[0])
    }

    /// The scroller `test_id` now stands at `(x, y)`.
    pub(crate) fn scroll(&mut self, test_id: &str, x: f64, y: f64) {
        let node = self.node(test_id);
        self.engine.set_scroll_offset(node, x, y);
    }

    /// What the engine presents for `test_id`'s `property` after a frame.
    pub(crate) fn shown(&mut self, test_id: &str, property: Property) -> exact_motion::Value {
        let node = self.node(test_id);
        self.engine.frame();
        self.engine
            .sampled_value(node, property)
            .unwrap_or_else(|| panic!("`{test_id}` shows no {property:?}"))
    }
}

/// #114's scorecard: the names row follows the columns' sideways scroll
/// through a named scroll timeline, its sibling's, in scope through their
/// parent.
const SCORECARD: &str = r#"keyframes follow
  to translate="-1000px 0px"
component App
  view
    column testId="sheet" timeline-scope="--cols"
      row testId="names" overflow="hidden" width=200
        row testId="track" flex-shrink=0 animation="follow 1s linear both" animation-timeline="--cols" animation-range="0px 1000px"
          for i in range(10) key=i
            text `Name ${i}` width=80 flex-shrink=0
      row testId="cols" overflow-x="auto" width=200 scroll-timeline="--cols x"
        for i in range(10) key=i
          text `${i}` width=80 flex-shrink=0
"#;

#[test]
fn a_named_scroll_timeline_moves_its_follower_with_the_scroller_offset() {
    let mut app = Laid::boot(SCORECARD, 400.0, 600.0);
    let k = app.runner.kernel();
    let (cols, track) = (k.find_by_test_id("cols")[0], k.find_by_test_id("track")[0]);
    assert_eq!(k.timeline_found(track), Some(Found::Scroll(cols)));
    let scroller = NamedTimeline::Scroll {
        scroller: motion_node(cols),
        x: true,
    };
    assert_eq!(k.timeline_of(track), Some(scroller));
    assert_eq!(
        k.timeline_scrollers(),
        &[TimelineScroller {
            view: k.node_by_key(cols).unwrap().id,
            node: motion_node(cols),
            page: false,
        }]
    );
    assert_eq!(app.shown("track", Property::Translate).x, 0.0);
    for x in [120.0, 340.5, 600.0] {
        app.scroll("cols", x, 0.0);
        assert_eq!(app.shown("track", Property::Translate).x, -x, "at {x}");
    }
    // The rubber band past the start: the range is unclamped, and `both`
    // holds the start.
    app.scroll("cols", -40.0, 0.0);
    assert_eq!(app.shown("track", Property::Translate).x, 0.0);
    // Placing again after a layout that moved nothing changes no binding.
    app.layout(400.0, 600.0);
    assert!(app.runner.kernel_mut().place_timelines().is_empty());
}

/// `normal` and percentages run over the scroll range, which is the content
/// less the scrollport; content that fits has none, and the timeline is
/// inactive: the node shows its own value.
#[test]
fn a_scroll_timeline_spans_its_scroll_range_and_is_inactive_without_one() {
    const APP: &str = r#"keyframes fade
  from opacity=0
  to opacity=1
component App
  state rows = 10
  view
    column testId="list" overflow-y="scroll" height=100 scroll-timeline-name="--list"
      box testId="normal" opacity=0.25 animation="fade 1s linear both" animation-timeline="--list"
      box testId="half" opacity=0.25 animation="fade 1s linear both" animation-timeline="--list" animation-range="50% normal"
      box testId="self" overflow="hidden" height=10 flex-shrink=0 opacity=0.25 animation="fade 1s linear both" animation-timeline="scroll(self)"
      for i in range(rows) key=i
        box height=50 flex-shrink=0
"#;
    let mut app = Laid::boot(APP, 300.0, 400.0);
    // Ten rows of 50 and two empty boxes and a 10-high one in a 100-high
    // scrollport: a scroll range of 410.
    app.scroll("list", 0.0, 205.0);
    assert!((app.shown("normal", Property::Opacity).x - 0.5).abs() < 1e-9);
    assert_eq!(app.shown("half", Property::Opacity).x, 0.0);
    app.scroll("list", 0.0, 307.5);
    assert!((app.shown("half", Property::Opacity).x - 0.5).abs() < 1e-9);
    // `scroll(self)` on a box that clips but holds nothing to scroll.
    assert_eq!(app.shown("self", Property::Opacity).x, 0.25);
    let k = app.runner.kernel();
    let own = k.find_by_test_id("self")[0];
    assert_eq!(k.timeline_found(own), Some(Found::Anonymous));
    assert_eq!(k.timeline_of(own), Some(NamedTimeline::Inactive));
}

/// `animation-timeline: none` gives the animation no timeline (CSS Animations
/// 2), so a fresh one holds time 0 without advancing on the clock (#2106),
/// `auto -> none -> auto` reveals the underlying style and keeps its clock
/// start, `scroll() -> none` keeps its progress, and `none`/`unset` do not
/// restrict animations to finite paint-only keyframes.
#[test]
fn animation_timeline_none_does_not_play_on_the_clock() {
    const APP: &str = r#"keyframes rise
  from opacity=0.2 translate="0px -10px"
  to opacity=0.8 translate="0px -50px"
keyframes grow
  from r=2
  to r=8
component App
  state mode = "none"
  action setMode(m: string)
    mode = m
  view
    column testId="page" padding=40 gap=20
      button "Auto" press=setMode("auto") testId="to-auto"
      button "None" press=setMode("none") testId="to-none"
      button "Scroll" press=setMode("scroll()") testId="to-scroll"
      row testId="none" width=80 height=40 opacity=1 animation="rise 1s linear both" animation-timeline="none"
      row testId="switch" width=80 height=40 opacity=1 animation="rise 1s linear both" animation-timeline=mode
      row testId="endless" width=80 height=40 animation="grow 1s linear infinite" animation-timeline=(mode == "auto" ? "unset" : "none")
      box height=1000 flex-shrink=0
"#;
    let mut app = Laid::boot(APP, 300.0, 400.0);
    let k = app.runner.kernel();
    let none = k.find_by_test_id("none")[0];
    assert_eq!(k.timeline_found(none), Some(Found::Missing));
    assert_eq!(k.timeline_of(none), Some(NamedTimeline::Missing));
    // Fresh `none` holds keyframe `from` (0.2 / -10px), distinct from underlying (1.0 / 0px),
    // and never advances on the clock.
    assert!((app.shown("none", Property::Opacity).x - 0.2).abs() < 1e-9);
    assert_eq!(app.shown("none", Property::Translate).y, -10.0);
    assert!((app.shown("switch", Property::Opacity).x - 0.2).abs() < 1e-9);
    app.engine.advance(2.0).unwrap();
    app.engine.frame();
    assert!(
        (app.engine
            .sampled_value(app.node("none"), Property::Opacity)
            .unwrap()
            .x
            - 0.2)
            .abs()
            < 1e-9
    );
    assert!(app.engine.quiescent());

    let press = |app: &mut Laid, id: &str| {
        let view = {
            let k = app.runner.kernel();
            k.node_by_key(k.find_by_test_id(id)[0]).unwrap().id
        };
        let receipt = app
            .runner
            .dispatch(view, exact_runner::Event::Press)
            .unwrap();
        app.runner
            .kernel()
            .motion_sync(&receipt)
            .apply(&mut app.engine)
            .unwrap();
        app.place();
    };

    // `none -> auto`: begins advancing from time 0 at t=2.0.
    press(&mut app, "to-auto");
    assert!((app.shown("switch", Property::Opacity).x - 0.2).abs() < 1e-9);
    app.engine.advance(2.4).unwrap();
    assert!((app.shown("switch", Property::Opacity).x - 0.44).abs() < 1e-9);

    // `auto -> none`: current time becomes unresolved -> underlying opacity 1.0.
    press(&mut app, "to-none");
    assert!((app.shown("switch", Property::Opacity).x - 1.0).abs() < 1e-9);
    assert!(app.engine.quiescent());

    // `none -> auto`: resumes keeping its t=2.0 start (at t=2.6 -> 60% -> 0.56).
    app.engine.advance(2.6).unwrap();
    press(&mut app, "to-auto");
    assert!((app.shown("switch", Property::Opacity).x - 0.56).abs() < 1e-9);

    // `scroll()` -> `none`: preserves the scroll timeline's progress.
    press(&mut app, "to-scroll");
    app.scroll("page", 0.0, 420.0);
    let scrolled = app.shown("switch", Property::Opacity).x;
    assert!(scrolled > 0.2 && scrolled < 0.8, "scrolled={scrolled}");
    press(&mut app, "to-none");
    app.scroll("page", 0.0, 0.0);
    app.engine.advance(5.0).unwrap();
    app.engine.frame();
    assert!(
        (app.engine
            .sampled_value(app.node("switch"), Property::Opacity)
            .unwrap()
            .x
            - scrolled)
            .abs()
            < 1e-9
    );
}

/// `scroll()` finds the nearest scroll container; with none above it, the
/// page's: a root the page scrolls, over the document less the viewport.
#[test]
fn an_anonymous_scroll_timeline_finds_its_nearest_scroller_or_the_page() {
    const APP: &str = r#"keyframes fade
  from opacity=0
  to opacity=1
component App
  view
    column testId="page"
      box testId="bar" height=4 flex-shrink=0 animation="fade 1s linear both" animation-timeline="scroll(root)"
      box testId="near" height=4 flex-shrink=0 animation="fade 1s linear both" animation-timeline="scroll()"
      column testId="inner" overflow-y="auto" height=100 flex-shrink=0
        box testId="inside" height=4 flex-shrink=0 animation="fade 1s linear both" animation-timeline="scroll(nearest)"
        box height=396 flex-shrink=0
      box height=992 flex-shrink=0
"#;
    let mut app = Laid::boot(APP, 300.0, 600.0);
    let k = app.runner.kernel();
    let page = k.roots()[0];
    let page = k.node(page).unwrap().key;
    let inner = k.find_by_test_id("inner")[0];
    let scrollers = k.timeline_scrollers().to_vec();
    assert_eq!(scrollers.len(), 2, "{scrollers:?}");
    assert!(scrollers
        .iter()
        .any(|s| s.node == motion_node(page) && s.page));
    assert!(scrollers
        .iter()
        .any(|s| s.node == motion_node(inner) && !s.page));
    // The document is 4 + 4 + 100 + 992 = 1100 tall in a 600 viewport.
    app.engine.set_scroll_offset(motion_node(page), 0.0, 250.0);
    assert!((app.shown("bar", Property::Opacity).x - 0.5).abs() < 1e-9);
    assert!((app.shown("near", Property::Opacity).x - 0.5).abs() < 1e-9);
    app.scroll("inner", 0.0, 75.0);
    assert!((app.shown("inside", Property::Opacity).x - 0.25).abs() < 1e-9);
}

#[test]
fn view_timeline_percentages_exclude_the_borders_and_scrollbar_gutter() {
    for inset in ["auto", "10% 6px"] {
        let source = format!(
            r#"keyframes fade
  from opacity=0
  to opacity=1
component App
  view
    scroll testId="port" box-sizing="border-box" width=300 height=300 overflow="scroll" direction="rtl" border-style="solid" border-width="8px 0 12px" scroll-padding-top="10%" scroll-padding-bottom=6
      box height=388
      box testId="subject" height=80 animation="fade 1s linear both" animation-timeline="view(y {inset})"
      box height=500
"#
        );
        let mut app = Laid::boot(&source, 400., 600.);
        let kernel = app.runner.kernel_mut();
        let id = kernel
            .node_by_key(kernel.find_by_test_id("port")[0])
            .unwrap()
            .id;
        kernel
            .set_host_cover(id, Some(exact_kernel::HostCover::Gutter([15., 20.])))
            .unwrap();
        app.layout(400., 600.);
        app.place();
        let k = app.runner.kernel();
        let scroller = k.find_by_test_id("port")[0];
        let port = k.laid_out_scrollport(scroller).unwrap();
        assert_eq!(
            (
                port.x - k.arena().frame(scroller.index).x,
                port.width,
                port.height
            ),
            (15., 285., 260.)
        );
        assert_eq!(
            k.arena().frame(k.find_by_test_id("subject")[0].index).y - port.y,
            388.
        );
        // The 260px port brings the subject into view at 388 - (260 - 6)
        // and takes it out of view at 468 - 26.
        app.scroll("port", 0., (134. + 442.) / 2.);
        assert!(
            (app.shown("subject", Property::Opacity).x - 0.5).abs() < 1e-9,
            "{inset}"
        );
    }
}

#[test]
fn a_horizontal_scroll_timeline_ends_at_the_guttered_ports_limit() {
    for direction in ["ltr", "rtl"] {
        let source = format!(
            r#"keyframes fade
  from opacity=0
  to opacity=1
component App
  view
    column
      scroll testId="port" direction="{direction}" width=300 height=300 box-sizing="border-box" overflow="scroll" border-style="solid" border-width="0 12px 0 8px" scroll-timeline="--p x"
        box testId="track" width=600 height=500 animation="fade 1s linear both" animation-timeline="--p"
"#
        );
        let mut app = Laid::boot(&source, 400., 600.);
        let k = app.runner.kernel_mut();
        let view = k.node_by_key(k.find_by_test_id("port")[0]).unwrap().id;
        for gutter in [0., 15.] {
            app.runner
                .kernel_mut()
                .set_host_cover(view, Some(exact_kernel::HostCover::Gutter([gutter, 0.])))
                .unwrap();
            app.layout(400., 600.);
            app.place();
            // Native hosts report x from the left (LLP 1057.003). Halfway along
            // the 600px track is halfway through the animation in either direction.
            app.scroll("port", (600. - 280. + f64::from(gutter)) / 2., 0.);
            assert!(
                (app.shown("track", Property::Opacity).x - 0.5).abs() < 1e-9,
                "{direction}, {gutter}"
            );
        }
    }
}
