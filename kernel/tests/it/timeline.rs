//! Drag timelines through the kernel (LLP 1057.003 D1, D2, D4): the four
//! rows parse as CSS text, refuse what is not their grammar, round-trip
//! through EXWF, and reach the engine with the node's other motion rows, so
//! a consumer follows its source's hold in the frame it moves, is handed
//! back to the clock when its row is cleared, and has no effect once the
//! source its parent's scope found is destroyed.

use exact_kernel::timeline::{AnimationTimeline, Axis, DragTimeline, TimelineScope};
use exact_kernel::{
    motion_node, wire, CommitReceipt, Kernel, NodeType, Op, StyleId, StyleProps, StyleValue,
    StyleValueError,
};
use exact_motion::{Engine, Property, Value};

const SOURCE: u32 = 2;
const CONSUMER: u32 = 3;

fn rows(pairs: &[(StyleId, &str)]) -> Box<StyleProps> {
    exact_kernel::timeline::link();
    let mut s = StyleProps::default();
    for (id, css) in pairs {
        s.set_dynamic(*id, &StyleValue::Text((*css).into()))
            .unwrap();
    }
    Box::new(s)
}

fn fade() -> Box<StyleProps> {
    let mut s = StyleProps::default();
    s.animation =
        crate::keyframed("fade 1s linear both @keyframes fade{from{opacity:1}to{opacity:0}}")
            .into();
    s.mask.set(StyleId::Animation);
    Box::new(s)
}

fn tree(k: &mut Kernel) -> CommitReceipt {
    let create = |id| Op::CreateView {
        id,
        node_type: NodeType::View,
    };
    k.apply(
        0,
        1,
        &[
            create(1),
            create(SOURCE),
            create(CONSUMER),
            // The two are siblings: their parent scopes the name (D4).
            Op::SetStyle {
                id: 1,
                patch: rows(&[(StyleId::TimelineScope, "--dismiss")]),
            },
            Op::SetStyle {
                id: SOURCE,
                patch: rows(&[(StyleId::DragTimeline, "--dismiss y")]),
            },
            Op::SetStyle {
                id: CONSUMER,
                patch: fade(),
            },
            Op::SetStyle {
                id: CONSUMER,
                patch: rows(&[
                    (StyleId::AnimationTimeline, "--dismiss"),
                    (StyleId::AnimationRange, "0px 300px"),
                ]),
            },
            Op::SetChildren {
                id: 1,
                children: vec![CONSUMER, SOURCE],
            },
            Op::AttachRoot { id: 1 },
        ],
    )
    .unwrap()
}

fn opacity(e: &mut Engine, node: u64) -> Option<f64> {
    e.frame()
        .iter()
        .find(|p| p.node == node && p.property == Property::Opacity)
        .map(|p| p.value.x)
}

#[test]
fn the_rows_are_css_text_and_refuse_what_is_not_their_grammar() {
    exact_kernel::timeline::link();
    let mut s = StyleProps::default();
    for (id, css, canonical) in [
        (StyleId::DragTimeline, "--dismiss", "--dismiss y"),
        (StyleId::DragTimeline, "--pan inline", "--pan x"),
        (StyleId::DragTimeline, "none", "none"),
        (StyleId::AnimationTimeline, "--dismiss", "--dismiss"),
        (StyleId::AnimationTimeline, "auto", "auto"),
        (StyleId::AnimationTimeline, "NONE", "none"),
        (StyleId::AnimationRange, "-300 300px", "-300px 300px"),
        (StyleId::AnimationRange, "normal", "normal"),
        (StyleId::TimelineScope, "--a,--b", "--a, --b"),
        (StyleId::TimelineScope, "all", "all"),
        (StyleId::TimelineScope, "none", "none"),
        (
            StyleId::AnimationTimeline,
            "scroll(x root)",
            "scroll(root x)",
        ),
        (StyleId::AnimationTimeline, "view(20%)", "view(20%)"),
        (StyleId::AnimationRange, "0px", "0px normal"),
        (StyleId::AnimationRange, "0% 100%", "normal"),
        (
            StyleId::AnimationRange,
            "exit -100% exit 0%",
            "exit -100% exit 0%",
        ),
        (StyleId::ScrollTimelineName, "--cols", "--cols"),
        (StyleId::ViewTimelineName, "none", "none"),
        (StyleId::ViewTimelineInset, "auto 10px", "auto 10px"),
    ] {
        s.set_dynamic(id, &StyleValue::Text(css.into())).unwrap();
        let written = match id {
            StyleId::DragTimeline => s.rare.drag_timeline.css(),
            StyleId::AnimationTimeline => s.rare.animation_timeline.css(),
            StyleId::TimelineScope => s.rare.timeline_scope.css(),
            StyleId::ScrollTimelineName => s.rare.scroll_timeline_name.css(),
            StyleId::ViewTimelineName => s.rare.view_timeline_name.css(),
            StyleId::ViewTimelineInset => s.rare.view_timeline_inset.css(),
            _ => s.animation_range.css(),
        };
        assert_eq!(written, canonical, "{css}");
        assert!(s.mask.has(id));
    }
    for (id, css) in [
        (StyleId::DragTimeline, "dismiss"),
        (StyleId::DragTimeline, "--dismiss z"),
        (StyleId::DragTimeline, "--dismiss y x"),
        (StyleId::AnimationTimeline, "scroll(far)"),
        (StyleId::AnimationTimeline, "--a, --b"),
        (StyleId::AnimationRange, "10px 10px"),
        (StyleId::AnimationRange, "exit 0% exit 0%"),
        (StyleId::AnimationRange, "exit 1em"),
        (StyleId::TimelineScope, "--a --b"),
        (StyleId::TimelineScope, "--a, all"),
        (StyleId::ScrollTimelineName, "--a, --b"),
        (StyleId::ViewTimelineName, "card"),
        (StyleId::ViewTimelineInset, "1px 2px 3px"),
    ] {
        let refused = s.set_dynamic(id, &StyleValue::Text(css.into()));
        assert!(
            matches!(
                refused,
                Err(StyleValueError::BadDragTimeline { .. }
                    | StyleValueError::BadAnimationTimeline { .. }
                    | StyleValueError::BadAnimationRange { .. }
                    | StyleValueError::BadTimelineScope { .. }
                    | StyleValueError::BadTimelineName { .. }
                    | StyleValueError::BadViewTimelineInset { .. })
            ),
            "{css}: {refused:?}"
        );
    }
}

#[test]
fn the_rows_round_trip_through_the_wire() {
    let mut k = Kernel::with_monospace();
    let ops = vec![
        Op::CreateView {
            id: 1,
            node_type: NodeType::View,
        },
        Op::SetStyle {
            id: 1,
            patch: rows(&[
                (StyleId::DragTimeline, "--pan x"),
                (StyleId::AnimationTimeline, "--dismiss"),
                (StyleId::AnimationRange, "-40px 260.5px"),
                (StyleId::TimelineScope, "--pan, --dismiss"),
                (StyleId::ScrollTimelineName, "--cols"),
                (StyleId::ScrollTimelineAxis, "x"),
                (StyleId::ViewTimelineName, "--card"),
                (StyleId::ViewTimelineInset, "10% auto"),
            ]),
        },
        Op::AttachRoot { id: 1 },
    ];
    k.apply_frame(&wire::encode(0, 1, &ops)).unwrap();
    let style = k.node(1).unwrap().style;
    assert_eq!(
        style.rare.drag_timeline,
        DragTimeline {
            name: Some("--pan".into()),
            axis: Axis::X
        }
    );
    assert_eq!(
        style.rare.animation_timeline,
        AnimationTimeline(Some("--dismiss".into()))
    );
    assert_eq!(style.animation_range.lengths(), Some([-40.0, 260.5]));
    assert_eq!(
        (
            style.rare.scroll_timeline_name.css(),
            style.rare.scroll_timeline_axis,
            style.rare.view_timeline_name.css(),
            style.rare.view_timeline_inset.css(),
        ),
        (
            "--cols".into(),
            exact_kernel::TimelineAxis::X,
            "--card".into(),
            "10% auto".into()
        )
    );
    assert_eq!(
        style.rare.timeline_scope,
        TimelineScope::Names("--pan, --dismiss".into())
    );
}

#[test]
fn a_consumer_follows_its_source_hold_until_its_row_is_cleared() {
    let mut k = Kernel::with_monospace();
    let receipt = tree(&mut k);
    let source = motion_node(k.node(SOURCE).unwrap().key);
    let consumer = motion_node(k.node(CONSUMER).unwrap().key);
    let mut e = Engine::new();
    k.motion_sync(&receipt).apply(&mut e).unwrap();
    assert!(e.timeline_bound(consumer));
    assert!(e.quiescent(), "a bound animation keeps no clock busy");
    // Held at the drag, in the frame that presents it.
    let hold = e
        .begin_hold(source, Property::Translate, 0.0, None)
        .unwrap()
        .unwrap();
    for (y, expected) in [(75.0, 0.75), (150.0, 0.5), (420.0, 0.0), (-30.0, 1.0)] {
        e.update_hold(hold.token, 0.1, Value::new(0.0, y)).unwrap();
        assert_eq!(opacity(&mut e, consumer), Some(expected), "at {y}");
    }
    e.update_hold(hold.token, 0.2, Value::new(0.0, 150.0))
        .unwrap();
    assert_eq!(opacity(&mut e, consumer), Some(0.5));
    // Clearing the row hands the animation back to the clock from there.
    let cleared = k
        .apply(
            1,
            1,
            &[Op::SetStyle {
                id: CONSUMER,
                patch: rows(&[(StyleId::AnimationTimeline, "auto")]),
            }],
        )
        .unwrap();
    k.motion_sync(&cleared).apply(&mut e).unwrap();
    assert!(!e.timeline_bound(consumer));
    e.advance(0.45).unwrap();
    let shown = opacity(&mut e, consumer).unwrap();
    assert!((shown - 0.25).abs() < 1e-9, "{shown}");
}

/// Chrome shows an inactive timeline's animation not in effect: the scope
/// that found the source finds none once it is destroyed.
#[test]
fn a_consumer_whose_scoped_source_is_destroyed_has_no_effect() {
    let mut k = Kernel::with_monospace();
    let receipt = tree(&mut k);
    let source = motion_node(k.node(SOURCE).unwrap().key);
    let consumer = motion_node(k.node(CONSUMER).unwrap().key);
    let mut e = Engine::new();
    k.motion_sync(&receipt).apply(&mut e).unwrap();
    let hold = e
        .begin_hold(source, Property::Translate, 0.0, None)
        .unwrap()
        .unwrap();
    e.update_hold(hold.token, 0.1, Value::new(0.0, 240.0))
        .unwrap();
    assert!((opacity(&mut e, consumer).unwrap() - 0.2).abs() < 1e-9);
    let gone = k
        .apply(
            1,
            1,
            &[
                Op::SetChildren {
                    id: 1,
                    children: vec![CONSUMER],
                },
                Op::DestroyView { id: SOURCE },
            ],
        )
        .unwrap();
    k.motion_sync(&gone).apply(&mut e).unwrap();
    assert!(e.timeline_bound(consumer), "still bound, to no source");
    assert_eq!(
        k.timeline_of(k.node(CONSUMER).unwrap().key),
        Some(exact_motion::NamedTimeline::Inactive)
    );
    // Its own opacity, 1, where the fade held 0.2.
    assert_eq!(opacity(&mut e, consumer), Some(1.0));
    e.advance(5.0).unwrap();
    assert_eq!(
        opacity(&mut e, consumer),
        None,
        "and the clock moves nothing"
    );
}

#[test]
fn default_scroll_containers_drive_descendant_timelines() {
    exact_kernel::timeline::link();
    for kind in [NodeType::ScrollView, NodeType::List] {
        for timeline in ["--scroll", "scroll()"] {
            let mut k = Kernel::with_monospace();
            let mut consumer_style = fade();
            for (id, css) in [
                (StyleId::Width, "100px"),
                (StyleId::Height, "100px"),
                (StyleId::AnimationTimeline, timeline),
                (StyleId::AnimationRange, "0px 200px"),
            ] {
                consumer_style
                    .set_dynamic(id, &StyleValue::Text(css.into()))
                    .unwrap();
            }
            let receipt = k
                .apply(
                    0,
                    1,
                    &[
                        Op::CreateView {
                            id: 1,
                            node_type: NodeType::View,
                        },
                        Op::CreateView {
                            id: 2,
                            node_type: kind,
                        },
                        Op::CreateView {
                            id: 3,
                            node_type: NodeType::View,
                        },
                        Op::CreateView {
                            id: 4,
                            node_type: NodeType::View,
                        },
                        Op::SetStyle {
                            id: 2,
                            patch: rows(&[
                                (StyleId::Width, "300px"),
                                (StyleId::Height, "500px"),
                                (StyleId::ScrollTimelineName, "--scroll"),
                            ]),
                        },
                        Op::SetStyle {
                            id: 3,
                            patch: rows(&[(StyleId::Height, "1400px")]),
                        },
                        Op::SetStyle {
                            id: 4,
                            patch: consumer_style,
                        },
                        Op::SetChildren {
                            id: 1,
                            children: vec![2],
                        },
                        Op::SetChildren {
                            id: 2,
                            children: vec![4, 3],
                        },
                        Op::AttachRoot { id: 1 },
                    ],
                )
                .unwrap();
            k.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
                .unwrap();
            let mut engine = Engine::new();
            k.motion_sync(&receipt).apply(&mut engine).unwrap();
            for (node, binding) in k.place_timelines() {
                engine.set_animation_timeline(node, binding);
            }
            let source = motion_node(k.node(2).unwrap().key);
            let consumer = motion_node(k.node(4).unwrap().key);
            engine.set_scroll_offset(source, 0., 100.);
            assert_eq!(
                opacity(&mut engine, consumer),
                Some(0.5),
                "{kind:?} {timeline}"
            );
            let scrollers = k.timeline_scrollers();
            assert_eq!(scrollers.len(), 1);
            assert_eq!(scrollers[0].view, 2);
            assert!(!scrollers[0].page);
        }
    }
}

#[test]
fn single_commit_none_switch_does_not_inherit_old_scroller_and_preserves_on_scroller_removal() {
    exact_kernel::timeline::link();
    exact_motion::link_animations();
    let fade_kf = exact_motion::Keyframes::parse("from{opacity:0}to{opacity:1}").unwrap();
    let slide_kf = exact_motion::Keyframes::parse("from{opacity:0.2}to{opacity:0.8}").unwrap();
    let anim = |text: &str| {
        let mut a = exact_motion::Animations::parse(text).unwrap();
        a.resolve(|n| match n {
            "fade" => Some(&fade_kf),
            "slide" => Some(&slide_kf),
            _ => None,
        });
        a
    };

    // Case 1: replacing `fade` with `slide` and switching `scroll() -> none` in one commit
    // must start `slide` at time 0 (0.2), never inheriting the old scroller's 75% progress.
    let mut k = Kernel::with_monospace();
    let mut consumer_style = rows(&[
        (StyleId::AnimationTimeline, "scroll()"),
        (StyleId::AnimationRange, "0px 200px"),
    ]);
    consumer_style.opacity = 0.9;
    consumer_style.mask.set(StyleId::Opacity);
    consumer_style.animation = anim("fade 1s linear both").into();
    consumer_style.mask.set(StyleId::Animation);
    let receipt = k
        .apply(
            0,
            1,
            &[
                Op::CreateView {
                    id: 1,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 2,
                    node_type: NodeType::ScrollView,
                },
                Op::CreateView {
                    id: 3,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 4,
                    node_type: NodeType::View,
                },
                Op::SetStyle {
                    id: 2,
                    patch: rows(&[(StyleId::Width, "300px"), (StyleId::Height, "500px")]),
                },
                Op::SetStyle {
                    id: 3,
                    patch: rows(&[(StyleId::Height, "1400px")]),
                },
                Op::SetStyle {
                    id: 4,
                    patch: consumer_style,
                },
                Op::SetChildren {
                    id: 1,
                    children: vec![2],
                },
                Op::SetChildren {
                    id: 2,
                    children: vec![4, 3],
                },
                Op::AttachRoot { id: 1 },
            ],
        )
        .unwrap();
    k.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
        .unwrap();
    let mut engine = Engine::new();
    k.motion_sync(&receipt).apply(&mut engine).unwrap();
    for (node, binding) in k.place_timelines() {
        engine.set_animation_timeline(node, binding);
    }
    let scroller = motion_node(k.node(2).unwrap().key);
    let consumer = motion_node(k.node(4).unwrap().key);
    engine.set_scroll_offset(scroller, 0., 150.); // 75% of 0..200
    assert!((opacity(&mut engine, consumer).unwrap() - 0.75).abs() < 1e-9);

    let mut swap = rows(&[(StyleId::AnimationTimeline, "none")]);
    swap.animation = anim("slide 1s linear both").into();
    swap.mask.set(StyleId::Animation);
    let receipt = k
        .apply(1, 2, &[Op::SetStyle { id: 4, patch: swap }])
        .unwrap();
    k.motion_sync(&receipt).apply(&mut engine).unwrap();
    assert!((opacity(&mut engine, consumer).unwrap() - 0.2).abs() < 1e-9);
    assert!(
        !k.has_timelines(),
        "explicit `none` needs no timeline registry entry"
    );

    // Case 2: removing a named scroller while switching its surviving consumer to `none`
    // in one commit must preserve the consumer's held 75% progress.
    let mut k2 = Kernel::with_monospace();
    let mut c2 = rows(&[
        (StyleId::AnimationTimeline, "--sc"),
        (StyleId::AnimationRange, "0px 200px"),
    ]);
    c2.opacity = 0.9;
    c2.mask.set(StyleId::Opacity);
    c2.animation = anim("fade 1s linear both").into();
    c2.mask.set(StyleId::Animation);
    let receipt = k2
        .apply(
            0,
            1,
            &[
                Op::CreateView {
                    id: 1,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 2,
                    node_type: NodeType::ScrollView,
                },
                Op::CreateView {
                    id: 3,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 4,
                    node_type: NodeType::View,
                },
                Op::SetStyle {
                    id: 1,
                    patch: rows(&[(StyleId::TimelineScope, "--sc")]),
                },
                Op::SetStyle {
                    id: 2,
                    patch: rows(&[
                        (StyleId::Width, "300px"),
                        (StyleId::Height, "500px"),
                        (StyleId::ScrollTimelineName, "--sc"),
                    ]),
                },
                Op::SetStyle {
                    id: 3,
                    patch: rows(&[(StyleId::Height, "1400px")]),
                },
                Op::SetStyle { id: 4, patch: c2 },
                Op::SetChildren {
                    id: 1,
                    children: vec![2, 4],
                },
                Op::SetChildren {
                    id: 2,
                    children: vec![3],
                },
                Op::AttachRoot { id: 1 },
            ],
        )
        .unwrap();
    k2.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
        .unwrap();
    let mut engine2 = Engine::new();
    k2.motion_sync(&receipt).apply(&mut engine2).unwrap();
    for (node, binding) in k2.place_timelines() {
        engine2.set_animation_timeline(node, binding);
    }
    let scroller2 = motion_node(k2.node(2).unwrap().key);
    let consumer2 = motion_node(k2.node(4).unwrap().key);
    engine2.set_scroll_offset(scroller2, 0., 150.); // 75%
    assert!((opacity(&mut engine2, consumer2).unwrap() - 0.75).abs() < 1e-9);

    let receipt = k2
        .apply(
            1,
            2,
            &[
                Op::SetChildren {
                    id: 1,
                    children: vec![4],
                },
                Op::SetChildren {
                    id: 2,
                    children: vec![],
                },
                Op::DestroyView { id: 3 },
                Op::DestroyView { id: 2 },
                Op::SetStyle {
                    id: 4,
                    patch: rows(&[(StyleId::AnimationTimeline, "none")]),
                },
            ],
        )
        .unwrap();
    k2.motion_sync(&receipt).apply(&mut engine2).unwrap();
    engine2.frame();
    assert!(
        (engine2
            .sampled_value(consumer2, Property::Opacity)
            .unwrap()
            .x
            - 0.75)
            .abs()
            < 1e-9
    );

    // Case 3: switching from scroller A (75%) to scroller B (20%) while replacing
    // the animation with a new paused animation in one commit must drop A's stale
    // placement so the new pause locks onto B (20%), never A (75%).
    let mut k3 = Kernel::with_monospace();
    let mut c3 = rows(&[
        (StyleId::AnimationTimeline, "--a"),
        (StyleId::AnimationRange, "0px 200px"),
    ]);
    c3.opacity = 0.9;
    c3.mask.set(StyleId::Opacity);
    c3.animation = anim("fade 1s linear both").into();
    c3.mask.set(StyleId::Animation);
    let receipt = k3
        .apply(
            0,
            1,
            &[
                Op::CreateView {
                    id: 1,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 2,
                    node_type: NodeType::ScrollView,
                },
                Op::CreateView {
                    id: 3,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 4,
                    node_type: NodeType::ScrollView,
                },
                Op::CreateView {
                    id: 5,
                    node_type: NodeType::View,
                },
                Op::CreateView {
                    id: 6,
                    node_type: NodeType::View,
                },
                Op::SetStyle {
                    id: 1,
                    patch: rows(&[(StyleId::TimelineScope, "--a, --b")]),
                },
                Op::SetStyle {
                    id: 2,
                    patch: rows(&[
                        (StyleId::Width, "300px"),
                        (StyleId::Height, "500px"),
                        (StyleId::ScrollTimelineName, "--a"),
                    ]),
                },
                Op::SetStyle {
                    id: 3,
                    patch: rows(&[(StyleId::Height, "1400px")]),
                },
                Op::SetStyle {
                    id: 4,
                    patch: rows(&[
                        (StyleId::Width, "300px"),
                        (StyleId::Height, "500px"),
                        (StyleId::ScrollTimelineName, "--b"),
                    ]),
                },
                Op::SetStyle {
                    id: 5,
                    patch: rows(&[(StyleId::Height, "1400px")]),
                },
                Op::SetStyle { id: 6, patch: c3 },
                Op::SetChildren {
                    id: 1,
                    children: vec![2, 4, 6],
                },
                Op::SetChildren {
                    id: 2,
                    children: vec![3],
                },
                Op::SetChildren {
                    id: 4,
                    children: vec![5],
                },
                Op::AttachRoot { id: 1 },
            ],
        )
        .unwrap();
    k3.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
        .unwrap();
    let mut engine3 = Engine::new();
    k3.motion_sync(&receipt).apply(&mut engine3).unwrap();
    for (node, binding) in k3.place_timelines() {
        engine3.set_animation_timeline(node, binding);
    }
    let scroller_a = motion_node(k3.node(2).unwrap().key);
    let scroller_b = motion_node(k3.node(4).unwrap().key);
    let consumer3 = motion_node(k3.node(6).unwrap().key);
    engine3.set_scroll_offset(scroller_a, 0., 150.); // 75%
    engine3.set_scroll_offset(scroller_b, 0., 40.); // 20%
    assert!((opacity(&mut engine3, consumer3).unwrap() - 0.75).abs() < 1e-9);

    let mut switch_paused = rows(&[(StyleId::AnimationTimeline, "--b")]);
    switch_paused.animation = anim("slide 1s linear both paused").into();
    switch_paused.mask.set(StyleId::Animation);
    let receipt = k3
        .apply(
            1,
            2,
            &[Op::SetStyle {
                id: 6,
                patch: switch_paused,
            }],
        )
        .unwrap();
    k3.motion_sync(&receipt).apply(&mut engine3).unwrap();
    k3.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
        .unwrap();
    for (node, binding) in k3.place_timelines() {
        engine3.set_animation_timeline(node, binding);
    }
    // `slide` is 0.2 -> 0.8; at B's 20% progress that is 0.2 + 0.2 * 0.6 = 0.32.
    assert!((opacity(&mut engine3, consumer3).unwrap() - 0.32).abs() < 1e-9);
    engine3.set_scroll_offset(scroller_b, 0., 180.);
    engine3.frame();
    assert!(
        (engine3
            .sampled_value(consumer3, Property::Opacity)
            .unwrap()
            .x
            - 0.32)
            .abs()
            < 1e-9
    );

    // Case 4: changing `animation-range` (0..200 -> 0..400) on the same scroller at offset 150
    // while installing a fresh paused animation in one commit must drop the stale range placement
    // so the pause locks at 37.5% (0.375), not 75% (0.75).
    engine3.set_scroll_offset(scroller_b, 0., 150.);
    let mut range_and_pause = rows(&[(StyleId::AnimationRange, "0px 400px")]);
    range_and_pause.animation = anim("fade 1s linear both paused").into();
    range_and_pause.mask.set(StyleId::Animation);
    let receipt = k3
        .apply(
            2,
            3,
            &[Op::SetStyle {
                id: 6,
                patch: range_and_pause,
            }],
        )
        .unwrap();
    k3.motion_sync(&receipt).apply(&mut engine3).unwrap();
    k3.compute_layout(1, exact_kernel::Offer::definite(300., 778.))
        .unwrap();
    for (node, binding) in k3.place_timelines() {
        engine3.set_animation_timeline(node, binding);
    }
    assert!((opacity(&mut engine3, consumer3).unwrap() - 0.375).abs() < 1e-9);
}
