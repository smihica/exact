use super::*;

fn marker_world() -> World {
    let mut w = World::new(8, 0);
    let mut model = world().model("rig.model").unwrap().clone();
    for clip in &mut model.clips {
        clip.markers = vec![
            (0., "start".into()),
            (clip.duration() * 0.5, "middle".into()),
            (clip.duration(), "end".into()),
        ];
    }
    w.assets.declared.insert("rig.model".into());
    w.assets
        .models
        .insert("rig.model".into(), crate::asset::ModelAsset::from(model));
    w
}

fn check_passes(w: &mut World, e: Entity, looping: bool, reverse: bool) {
    let mut counts = [0; 3];
    for tick in 0..24 {
        step(w);
        if tick == 0 {
            let start = if reverse && !looping { "end" } else { "start" };
            assert_eq!(w.get::<Pose>(e).unwrap().crossed, [start]);
            let logs = w.journal().len();
            step(w);
            assert_eq!(w.journal().len(), logs, "same-tick step repeated a marker");
        }
        for (count, name) in counts.iter_mut().zip(["start", "middle", "end"]) {
            *count += usize::from(w.get::<Pose>(e).unwrap().crossed.contains(&name.into()));
        }
        w.step_clock();
    }
    assert_eq!(counts, if looping { [7, 6, 6] } else { [1, 1, 1] });
}

#[test]
fn single_clip_includes_its_starting_marker() {
    for looping in [false, true] {
        for reverse in [false, true] {
            let mut w = marker_world();
            let mut a = Animation::play("fast");
            a.looping = looping;
            a.speed = if reverse { -1. } else { 1. };
            // Baked and authored copies of a marker still emit one event.
            a = a.marker(0., "start").marker(0.5, "end");
            let e = w.spawn((Mesh::asset("rig.model"), a));
            check_passes(&mut w, e, looping, reverse);
        }
    }
}

#[test]
fn layer_includes_its_starting_marker() {
    for looping in [false, true] {
        for reverse in [false, true] {
            let mut w = marker_world();
            let mut a = Animation::play("fast");
            a.looping = looping;
            a.speed = if reverse { -1. } else { 1. };
            let e = w.spawn((Mesh::asset("rig.model"), Layers(vec![Layer::new(a)])));
            check_passes(&mut w, e, looping, reverse);
        }
    }
}

#[test]
fn same_tick_redelivery_keeps_new_layer_startup() {
    for reverse in [false, true] {
        let mut w = marker_world();
        let e = w.spawn_named("actor", (Mesh::asset("rig.model"), Ik::default()));
        step(&mut w);
        let logs = w.journal().len();
        w.insert(
            e,
            Layers(vec![Layer::new(
                Animation::play("fast")
                    .once()
                    .speed(if reverse { -1. } else { 1. }),
            )]),
        );
        let starting = if reverse { "end" } else { "start" };
        let event = format!(" animation actor blend {starting}");
        let before = crate::bin::to_vec(&*w.get::<Layers>(e).unwrap());
        for _ in 0..2 {
            let mut model = w.model("rig.model").unwrap().clone();
            model.clips[1].tracks[0].values[0] = 10.;
            model.clips[1].tracks[0].values[3] = 20.;
            w.assets.models.insert("rig.model".into(), model.into());
            let motion = step(&mut w);
            assert!(!motion.crossed(e, starting));
            assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
            assert_eq!(w.journal().len(), logs);
            assert_eq!(
                w.get::<Pose>(e).unwrap().local[0],
                if reverse { 20. } else { 10. }
            );
            assert_eq!(crate::bin::to_vec(&*w.get::<Layers>(e).unwrap()), before);
        }
        for tick in 0..5 {
            w.step_clock();
            let motion = step(&mut w);
            assert_eq!(motion.crossed(e, starting), tick == 0);
            assert_eq!(
                w.get::<Layers>(e).unwrap().0[0].animation.crossed(starting),
                tick == 0
            );
            assert_eq!(
                w.get::<Pose>(e).unwrap().crossed.contains(&starting.into()),
                tick == 0
            );
            assert_eq!(
                w.journal()
                    .iter()
                    .filter(|line| line.line.ends_with(&event))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn same_tick_redelivery_preserves_layer_markers() {
    let mut w = marker_world();
    let e = w.spawn((
        Mesh::asset("rig.model"),
        Layers(vec![Layer::new(Animation::play("fast").once())]),
    ));
    step(&mut w);
    w.step_clock();
    assert!(step(&mut w).crossed(e, "middle"));
    let before = crate::bin::to_vec(&*w.get::<Layers>(e).unwrap());
    let logs = w.journal().len();
    let mut model = w.model("rig.model").unwrap().clone();
    model.clips[1].tracks[0].values[3] = 2.;
    w.assets.models.insert("rig.model".into(), model.into());
    assert!(step(&mut w).crossed(e, "middle"));
    assert_eq!(w.get::<Pose>(e).unwrap().local[0], 1.);
    assert_eq!(w.get::<Pose>(e).unwrap().crossed, ["middle"]);
    assert_eq!(w.journal().len(), logs);
    assert!(w.get::<Layers>(e).unwrap().0[0].animation.crossed("middle"));
    assert_eq!(crate::bin::to_vec(&*w.get::<Layers>(e).unwrap()), before);
    w.step_clock();
    assert!(!step(&mut w).crossed(e, "middle"));
    assert!(!w.get::<Layers>(e).unwrap().0[0].animation.crossed("middle"));
    assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
    assert_eq!(w.journal().len(), logs);
}

#[test]
fn animator_includes_its_starting_marker() {
    for looping in [false, true] {
        for reverse in [false, true] {
            let mut w = marker_world();
            let mut state = State::clip("play", "fast");
            state.looping = looping;
            state.speed = if reverse { -1. } else { 1. };
            let e = w.spawn((Mesh::asset("rig.model"), Animator::new([state])));
            check_passes(&mut w, e, looping, reverse);
        }
    }
}

#[test]
fn blend_includes_its_starting_marker() {
    let mut w = marker_world();
    let e = w.spawn((Mesh::asset("rig.model"), Blend::across([(0., "fast")])));
    check_passes(&mut w, e, true, false);
}

#[test]
fn attached_blend_keeps_its_starting_marker_across_restore() {
    for restore_before_start in [false, true] {
        let mut w = marker_world();
        let e = w.spawn((Mesh::asset("rig.model"), Ik::default()));
        step(&mut w);
        assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
        w.step_clock();
        w.insert(e, Blend::across([(0., "fast")]));
        if restore_before_start {
            w.load(&w.save()).unwrap();
        }
        check_passes(&mut w, e, true, false);
        assert_eq!(w.get::<Pose>(e).unwrap().phase, 0.);
        w.load(&w.save()).unwrap();
        step(&mut w);
        assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
    }
}

#[test]
fn animator_loop_transition_does_not_repeat_a_departure_marker() {
    for speed in [0., 1.] {
        let mut w = marker_world();
        let e = w.spawn((
            Mesh::asset("rig.model"),
            Animator::new([
                State::clip("walk", "fast")
                    .speed(speed)
                    .to("run", Condition::Arg("go".into(), Cmp::Eq, true.into())),
                State::clip("run", "fast").speed(speed),
            ]),
        ));
        for _ in 0..if speed == 0. { 1 } else { 4 } {
            step(&mut w);
            w.step_clock();
        }
        assert_eq!(w.get::<Pose>(e).unwrap().phase, 0.);
        assert!(w.get::<Animator>(e).unwrap().crossed("start"));
        w.get_mut::<Animator>(e).unwrap().set("go", true);
        step(&mut w);
        assert_eq!(w.get::<Animator>(e).unwrap().state(), "run");
        assert!(
            w.get::<Pose>(e).unwrap().crossed.is_empty(),
            "speed {speed}"
        );
        w.step_clock();
        step(&mut w);
        assert!(!w.get::<Animator>(e).unwrap().crossed("start"));
    }
}

#[test]
fn animator_paused_transition_preserves_marker_startup_across_restore() {
    for (looping, restore) in [(true, false), (true, true), (false, false), (false, true)] {
        let mut w = marker_world();
        let mut incoming = State::clip("run", "fast").paused(true);
        incoming.looping = looping;
        let e = w.spawn((
            Mesh::asset("rig.model"),
            Animator::new([
                State::clip("walk", "fast")
                    .to("run", Condition::Arg("go".into(), Cmp::Eq, true.into())),
                incoming,
            ]),
        ));
        for _ in 0..4 {
            step(&mut w);
            w.step_clock();
        }
        assert_eq!(w.get::<Pose>(e).unwrap().phase, 0.);
        assert!(w.get::<Animator>(e).unwrap().crossed("start"));
        w.get_mut::<Animator>(e).unwrap().set("go", true);
        step(&mut w);
        assert_eq!(w.get::<Animator>(e).unwrap().state(), "run");
        assert_eq!(w.get::<Animator>(e).unwrap().since, 0.);
        assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
        w.step_clock();
        if restore {
            w.load(&w.save()).unwrap();
        }
        step(&mut w);
        assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
        w.step_clock();
        w.get_mut::<Animator>(e)
            .unwrap()
            .state_mut("run")
            .unwrap()
            .paused = false;
        for tick in 0..4 {
            step(&mut w);
            assert_eq!(
                w.get::<Animator>(e).unwrap().crossed("start"),
                if looping { tick == 3 } else { tick == 0 },
                "tick {tick}, looping {looping}, restore {restore}"
            );
            w.step_clock();
        }
    }
}

#[test]
fn animator_start_markers_wait_for_resume_and_fire_on_each_state_entry() {
    let mut w = marker_world();
    let ready = Condition::Arg("ready".into(), Cmp::Eq, true.into());
    let mut animator = Animator::new([
        State::blend("forward", Blend::across([(0., "fast")]))
            .once()
            .paused(true)
            .to("reverse", ready.clone()),
        State::clip("reverse", "fast")
            .once()
            .speed(-1.)
            .to("forward", ready),
    ]);
    animator.set("ready", true);
    let e = w.spawn((Mesh::asset("rig.model"), animator));
    for _ in 0..2 {
        step(&mut w);
        assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
        w.step_clock();
    }
    w.get_mut::<Animator>(e)
        .unwrap()
        .state_mut("forward")
        .unwrap()
        .paused = false;
    for tick in 0..12 {
        step(&mut w);
        let reverse = tick / 4 == 1;
        let expected: &[&str] = match tick % 4 {
            0 if reverse => &["end"],
            0 => &["start"],
            1 => &["middle"],
            3 if reverse => &["start"],
            3 => &["end"],
            _ => &[],
        };
        assert_eq!(w.get::<Pose>(e).unwrap().crossed, expected, "tick {tick}");
        w.step_clock();
    }
}

#[test]
fn stationary_start_markers_do_not_repeat_after_restore() {
    let mut w = marker_world();
    let single = w.spawn((Mesh::asset("rig.model"), Animation::play("fast").speed(0.)));
    let layered = w.spawn((
        Mesh::asset("rig.model"),
        Layers(vec![Layer::new(Animation::play("fast").speed(0.))]),
    ));
    let animator = w.spawn((
        Mesh::asset("rig.model"),
        Animator::new([State::clip("play", "fast").speed(0.)]),
    ));
    step(&mut w);
    for e in [single, layered, animator] {
        assert_eq!(w.get::<Pose>(e).unwrap().crossed, ["start"]);
    }
    w.step_clock();
    w.load(&w.save()).unwrap();
    for _ in 0..2 {
        step(&mut w);
        for e in [single, layered, animator] {
            assert!(w.get::<Pose>(e).unwrap().crossed.is_empty());
        }
        w.step_clock();
    }
}

#[test]
fn start_markers_respect_layer_weight_and_state_fades() {
    let mut w = marker_world();
    let layered = w.spawn((
        Mesh::asset("rig.model"),
        Layers(vec![Layer::new(Animation::play("fast")).weight(0.)]),
    ));
    let animator = w.spawn((
        Mesh::asset("rig.model"),
        Animator::new([
            State::clip("idle", "slow")
                .to("attack", Condition::Arg("go".into(), Cmp::Eq, true.into())),
            State::clip("attack", "fast").once().fade(0.5),
        ]),
    ));
    step(&mut w);
    assert!(w.get::<Pose>(layered).unwrap().crossed.is_empty());
    w.step_clock();
    w.get_mut::<Animator>(animator).unwrap().set("go", true);
    for tick in 0..4 {
        step(&mut w);
        let a = w.get::<Animator>(animator).unwrap();
        assert_eq!(a.state(), "attack");
        assert!(!a.crossed("start"));
        if tick <= 1 {
            assert!(w.get::<Pose>(animator).unwrap().crossed.is_empty());
        }
        drop(a);
        w.step_clock();
    }
}
