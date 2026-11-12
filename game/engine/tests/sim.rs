use exact_game::*;

#[derive(Default, Resource)]
struct Counts {
    held: u32,
    pressed: u32,
    released: u32,
    distance: Vec2,
    pointer: Vec2,
    wheel: Vec2,
}
struct Counter;
#[derive(Default, exact_game::Args)]
struct CounterArgs {
    /// Canvas live argument.
    #[live]
    pub paused: bool,
}
impl Game for Counter {
    const ID: &'static str = "Counter";
    type Args = CounterArgs;

    fn setup(w: &mut World, _: &Self::Args) {
        w.insert_resource(Counts::default());
        w.spawn_named("counter", (Transform::default(), Ambient));
    }
    fn actions() -> Actions {
        Actions::new()
            .button("act", &["KeyE", "Enter"])
            .button_touch("act", Region::Right)
            .stick(
                "move",
                Stick::keys("KeyW", "KeyS", "KeyA", "KeyD").or_touch(Region::Left),
            )
    }
    fn paused(args: &Self::Args) -> bool {
        args.paused
    }
    fn tick(w: &mut World, i: &Input, _: &Self::Args) {
        let mut c = w.resource_mut::<Counts>();
        c.held += u32::from(i.held("act"));
        c.pressed += u32::from(i.pressed("act"));
        c.released += u32::from(i.released("act"));
        c.distance += i.stick("move");
        c.wheel += i.wheel();
        if let Some(p) = i.pointer() {
            c.pointer += p.delta;
        }
        w.get_mut::<Transform>(w.named("counter").unwrap())
            .unwrap()
            .position
            .x += 1.0;
    }
}
fn sim() -> Sim<Counter> {
    let mut s = Sim::new(CounterArgs::default()).unwrap();
    s.advance(0.0, Clock::Seekable);
    s
}
fn key(s: &mut Sim<Counter>, code: &str, down: bool, at_ms: f64) {
    s.input(InputEvent::Key {
        code: code.into(),
        down,
        at_ms,
        player: 0,
    });
}
#[test]
fn integer_seek_partitions_and_fraction() {
    let mut a = sim();
    let mut b = sim();
    let mut c = sim();
    for s in [&mut a, &mut b, &mut c] {
        key(s, "KeyW", true, 0.0);
        key(s, "KeyE", true, 134.123);
        key(s, "KeyE", false, 721.987);
    }
    a.advance(1000.0, Clock::Seekable);
    for ms in 1..=1000 {
        b.advance(ms as f64, Clock::Seekable);
    }
    for ms in [0.123, 2.007, 15.1, 100.0, 501.3, 789.999, 999.001, 1000.0] {
        c.advance(ms, Clock::Seekable);
    }
    assert_eq!(a.world().hash(), b.world().hash());
    assert_eq!(a.world().hash(), c.world().hash());
    assert_eq!(a.world().tick(), 60);
    assert_eq!(a.alpha(), 0.0);
    a.advance(1008.333, Clock::Seekable);
    assert!((a.alpha() - 0.49998).abs() < 1e-5);
    let e = a.world().named("counter").unwrap();
    assert_eq!(a.world().global(e).unwrap().translation.x, 60.0);
}
#[test]
fn exact_event_boundary_tap_repeat_alias_and_blur() {
    let mut s = sim();
    key(&mut s, "KeyE", true, 1.0);
    key(&mut s, "KeyE", false, 2.0);
    s.advance(16.667, Clock::Seekable);
    assert_eq!(s.world().resource::<Counts>().pressed, 1);
    s.advance(33.334, Clock::Seekable);
    {
        let c = s.world().resource::<Counts>();
        assert_eq!((c.held, c.pressed, c.released), (0, 1, 1));
    }
    key(&mut s, "KeyE", true, 40.0);
    key(&mut s, "KeyE", true, 41.0);
    key(&mut s, "Enter", true, 42.0);
    key(&mut s, "KeyE", false, 43.0);
    s.advance(100.0, Clock::Seekable);
    {
        let c = s.world().resource::<Counts>();
        assert_eq!((c.pressed, c.released), (2, 1));
    }
    s.input(InputEvent::Blur {
        at_ms: 100.0,
        player: 0,
    });
    s.advance(116.667, Clock::Seekable);
    assert_eq!(s.world().resource::<Counts>().released, 2);
    // 60 Hz's rational boundary at 16,666 2/3 us is not rounded down.
    let mut before = sim();
    key(&mut before, "KeyE", true, 16.666);
    before.advance(16.667, Clock::Seekable);
    assert_eq!(before.world().resource::<Counts>().pressed, 1);
    let mut after = sim();
    key(&mut after, "KeyE", true, 16.667);
    after.advance(16.667, Clock::Seekable);
    assert_eq!(after.world().resource::<Counts>().pressed, 0);
    after.advance(33.334, Clock::Seekable);
    assert_eq!(after.world().resource::<Counts>().pressed, 1);
}
#[test]
fn epoch_pause_live_gap_and_backwards_clock() {
    let mut s = Sim::<Counter>::new(CounterArgs::default()).unwrap();
    assert_eq!(s.advance(5000.0, Clock::Seekable), 0);
    key(&mut s, "KeyE", true, 5100.0);
    assert_eq!(s.advance(6000.0, Clock::Live), 15);
    assert_eq!(s.advance(5500.0, Clock::Seekable), 0);
    assert_eq!(s.advance(6000.0, Clock::Seekable), 0);
    s.bind(&CounterArgs { paused: true }.values(), None)
        .unwrap();
    key(&mut s, "KeyE", false, 6500.0);
    s.advance(7000.0, Clock::Seekable);
    assert_eq!(s.world().tick(), 15);
    s.bind(&CounterArgs::default().values(), None).unwrap();
    s.advance(7100.0, Clock::Seekable);
    let c = s.world().resource::<Counts>();
    assert_eq!(c.released, 0);
    assert_eq!(s.world().tick(), 21);
}
#[test]
fn touch_is_data_and_pointer_wheel_deltas_expire() {
    let mut s = sim();
    s.viewport(800.0, 600.0);
    for (id, phase, x, y, dx, dy) in [
        (1, PointerPhase::Down, 100.0, 300.0, 0.0, 0.0),
        (1, PointerPhase::Move, 160.0, 240.0, 60.0, -60.0),
        (2, PointerPhase::Down, 700.0, 300.0, 0.0, 0.0),
    ] {
        s.input(InputEvent::Pointer {
            id,
            phase,
            x,
            y,
            dx,
            dy,
            buttons: 0,
            at_ms: 0.0,
            player: 0,
        });
    }
    s.input(InputEvent::Wheel {
        dx: 2.0,
        dy: -3.0,
        at_ms: 0.0,
        player: 0,
    });
    s.advance(16.667, Clock::Seekable);
    {
        let c = s.world().resource::<Counts>();
        assert!((c.distance.length() - 1.0).abs() < 1e-6);
        assert_eq!(c.pointer, Vec2::new(60.0, -60.0));
        assert_eq!(c.wheel, Vec2::new(2.0, -3.0));
        assert_eq!(c.pressed, 1);
    }
    s.advance(33.334, Clock::Seekable);
    assert_eq!(
        s.world().resource::<Counts>().pointer,
        Vec2::new(60.0, -60.0)
    );
    assert!(s.agent(r#"{"op":"state"}"#).contains("Right"));
    s.input(InputEvent::Blur {
        at_ms: 34.0,
        player: 0,
    });
    s.advance(100.0, Clock::Seekable);
    assert_eq!(s.world().resource::<Counts>().released, 1);
}
/// Moves coalesced inside one tick keep every event's device motion: a locked
/// pointer's position never changes, so a position difference would lose it.
#[test]
fn coalesced_moves_keep_their_motion_and_the_first_move_counts() {
    let mut s = sim();
    for (at_ms, dx) in [(1.0, 5.0), (2.0, 7.0), (3.0, -2.0)] {
        s.input(InputEvent::Pointer {
            id: 1,
            phase: PointerPhase::Move,
            x: 40.0,
            y: 30.0,
            dx,
            dy: 1.0,
            buttons: 0,
            at_ms,
            player: 0,
        });
    }
    s.advance(16.667, Clock::Seekable);
    assert_eq!(s.world().resource::<Counts>().pointer, Vec2::new(10.0, 3.0));
}
#[derive(Default, Component)]
struct Nested {
    springs: Vec<Option<Box<Spring>>>,
}
#[test]
fn settle_finds_nested_springs_and_busy_is_per_tick() {
    let mut s = sim();
    let mut spring = Spring::new(0.0);
    spring.set_target(s.world().now(), 1.0);
    s.world_mut().spawn(Nested {
        springs: vec![Some(Box::new(spring))],
    });
    assert!(!s.quiescent());
    settle_host(&mut s);
    assert!(s.quiescent());
    s.world_mut().busy("one step");
    assert!(!s.quiescent());
    settle_host(&mut s);
    assert!(s.quiescent());
}
#[test]
fn publication_drain_is_full_and_journal_is_an_indexed_ring() {
    let mut s = sim();
    s.world().publish("a", 2);
    s.world().publish("b", false);
    assert_eq!(s.take_published().as_deref(), Some("{\"a\":2,\"b\":false}"));
    s.world().publish("a", 2);
    assert!(s.take_published().is_none());
    s.world().publish("b", true);
    assert_eq!(s.take_published().as_deref(), Some("{\"a\":2,\"b\":true}"));
    for i in 0..5000 {
        s.world().log(format_args!("line {i}"));
    }
    let next = s.world().journal_next();
    assert_eq!(s.world().journal().len(), 4096);
    let reply = s.agent(r#"{"op":"logs","since":0}"#);
    assert!(reply.contains(&format!("\"from\":{}", next - 4096)));
    assert!(reply.contains(&format!("\"dropped\":{}", next - 4096)));
    assert_eq!(
        s.agent(&format!("{{\"op\":\"logs\",\"since\":{next}}}")),
        format!("{{\"tick\":0,\"next\":{next},\"from\":{next},\"dropped\":0,\"lines\":[]}}")
    );
}
#[test]
fn named_refusals_and_reads_do_not_change_the_hash() {
    assert!(Sim::<Counter>::from_values(&[]).is_ok());
    assert!(Sim::<Counter>::from_values(&[Value::Number(1.0)])
        .err()
        .unwrap()
        .contains("paused"));
    let mut s = sim();
    let hash = s.world().hash();
    for q in [
        r#"{"op":"tree"}"#,
        r#"{"op":"state","world":true}"#,
        r#"{"op":"layout","entity":"counter"}"#,
        r#"{"op":"missing"}"#,
        r#"{"op":"state","entity":"nope"}"#,
    ] {
        s.agent(q);
    }
    assert_eq!(s.world().hash(), hash);
    assert!(s
        .agent(r#"{"op":"layout","entity":"counter"}"#)
        .contains("\"unavailable\":true"));
}
#[test]
#[should_panic(expected = "unknown action `typo`")]
fn misspelled_actions_are_not_silent() {
    Input::default().held("typo");
}

#[test]
fn a_late_event_cannot_block_an_earlier_pending_boundary() {
    let mut s = sim();
    key(&mut s, "KeyE", true, 10.0);
    s.advance(24.0, Clock::Seekable);
    key(&mut s, "KeyE", false, 0.0); // late delivery maps to world 24 ms
    s.advance(33.334, Clock::Seekable);
    {
        let c = s.world().resource::<Counts>();
        assert_eq!((c.pressed, c.released), (1, 1));
    }
    s.advance(50.0, Clock::Seekable);
    assert_eq!(s.world().resource::<Counts>().released, 1);
}
#[test]
fn settle_is_bounded_for_continuous_game_work() {
    let mut s = sim();
    #[derive(Default, Component)]
    struct Never {
        spring: Spring,
    }
    let mut spring = Spring {
        config: SpringConfig {
            stiffness: 1.0,
            damping: 0.0,
            mass: 1.0,
        },
        ..Spring::new(0.0)
    };
    spring.set_target(s.world().now(), 1.0);
    s.world_mut().spawn(Never { spring });
    let reply = s.agent(r#"{"op":"clock","settle":true}"#);
    assert_eq!(s.world().tick(), 0);
    assert!(reply.contains("\"quiescent\":false"));
}

fn settle_host(s: &mut Sim<Counter>) {
    #[allow(non_snake_case)]
    #[derive(Default, Data)]
    struct Reply {
        quiescent: bool,
        settleAt: f64,
    }
    for _ in 0..16 {
        let reply: Reply = json::from_str(&s.agent(r#"{"op":"clock","settle":true}"#)).unwrap();
        if reply.quiescent {
            return;
        }
        s.advance(reply.settleAt, Clock::Seekable);
    }
    panic!("world did not settle");
}

#[test]
fn explicit_events_are_saved_in_order_and_publication_is_separate() {
    let mut s = sim();
    let empty_save = s.world().save();
    let empty_hash = s.world().hash();
    assert_eq!(s.take_published().as_deref(), Some("{}"));
    s.world().publish("count", 4);
    s.world().emit("first");
    s.world().emit("second");
    assert_eq!(s.world().hash(), empty_hash);
    let saved = s.save().unwrap();
    let mut restored = sim();
    restored.restore(&saved).unwrap();
    assert_eq!(s.take_messages(), ["first", "second"]);
    assert_eq!(restored.take_messages(), ["first", "second"]);
    assert!(restored.take_messages().is_empty());
    assert_eq!(
        s.world().save(),
        empty_save,
        "empty event queue adds no save bytes"
    );
    assert_eq!(s.world().hash(), empty_hash);
    assert_eq!(restored.take_published().as_deref(), Some("{\"count\":4}"));
    assert_eq!(s.take_published(), Some("{\"count\":4}".into()));
    let saved = s.save().unwrap(); // publication was already delivered before this save
    restored.restore(&saved).unwrap();
    assert_eq!(restored.take_published().as_deref(), Some("{\"count\":4}"));
    assert!(restored.take_published().is_none());
    assert_eq!(
        restored.save().unwrap(),
        saved,
        "delivery cursor is not simulation state"
    );
}

/// An overflowing queue drops its oldest move, but that move's motion joins the
/// pointer's next event: 1,100 one-point moves in 1,100 ticks still turn 1,100.
/// Before: 1,024 (the 76 dropped moves' motion was lost).
#[test]
fn overflow_drops_a_moves_position_but_keeps_its_motion() {
    let mut s = sim();
    for i in 0..1100 {
        s.input(InputEvent::Pointer {
            id: 1,
            phase: PointerPhase::Move,
            x: i as f32,
            y: 0.0,
            dx: 1.0,
            dy: 0.0,
            buttons: 0,
            at_ms: 20.0 * (i + 1) as f64,
            player: 0,
        });
    }
    s.advance(30_000.0, Clock::Seekable);
    assert_eq!(
        s.world().resource::<Counts>().pointer,
        Vec2::new(1100.0, 0.0)
    );
}
#[test]
fn overflow_warning_is_saved_behavior() {
    let mut a = sim();
    for i in 0..1030 {
        key(&mut a, "KeyE", i % 2 == 0, 0.0);
    }
    let mut b = sim();
    b.restore(&a.save().unwrap()).unwrap();
    b.advance(0.0, Clock::Seekable);
    for i in 1030..1040 {
        key(&mut a, "KeyE", i % 2 == 0, 0.0);
        key(&mut b, "KeyE", i % 2 == 0, 0.0);
    }
    assert_eq!(a.save().unwrap(), b.save().unwrap());
}

#[test]
fn older_saves_default_the_overflow_warning_flag() {
    let s = sim();
    let mut bytes = s.save().unwrap();
    let field = b"\x01\x00\x0foverflow_logged";
    let at = bytes.windows(field.len()).position(|v| v == field).unwrap();
    bytes.drain(at..at + field.len() + 1);
    let mut restored = sim();
    restored.restore(&bytes).unwrap();
    assert_eq!(s.save().unwrap(), restored.save().unwrap());
}

#[test]
fn forwarded_keys_include_pending_downs_and_releases_without_changing_tick_held_state() {
    let mut s = sim();
    key(&mut s, "KeyE", true, 0.0);
    let state = s.agent(r#"{"op":"state"}"#);
    assert!(state.contains(r#""held":[],"forwarded":["KeyE"]"#));
    key(&mut s, "KeyE", false, 0.0);
    assert!(s.agent(r#"{"op":"state"}"#).contains(r#""forwarded":[]"#));
}

#[test]
fn agent_input_projection_preserves_pending_contacts_pause_and_restore() {
    fn input_state(s: &mut Sim<Counter>) -> String {
        let saved = s.save().unwrap();
        let hash = s.world().hash();
        let tick = s.world().tick();
        let state = s.agent(r#"{"op":"state"}"#);
        assert_eq!(s.agent(r#"{"op":"state"}"#), state);
        assert_eq!(s.save().unwrap(), saved);
        assert_eq!(s.world().hash(), hash);
        assert_eq!(s.world().tick(), tick);
        let input = state
            .split_once("\"input\":")
            .unwrap()
            .1
            .split_once(",\"published\":")
            .unwrap()
            .0;
        // The last tick's edges are not saved: a restored world has fired nothing.
        let (before, fired) = input.split_once(",\"fired\":").unwrap();
        format!("{before}{}", &fired[fired.find('}').unwrap() + 1..])
    }
    let mut s = sim();
    key(&mut s, "KeyZ", true, 0.);
    key(&mut s, "KeyA", true, 0.);
    s.advance(17., Clock::Seekable);
    key(&mut s, "KeyZ", false, 18.);
    key(&mut s, "KeyM", true, 20.);
    key(&mut s, "KeyA", true, 22.);
    for (id, phase, at_ms) in [
        (7, PointerPhase::Down, 23.),
        (8, PointerPhase::Down, 24.),
        (7, PointerPhase::Move, 25.),
        (8, PointerPhase::Cancel, 26.),
    ] {
        s.input(InputEvent::Control {
            name: "act".into(),
            id,
            phase,
            x: at_ms as f32,
            y: 10.,
            at_ms,
            player: 0,
        });
    }
    let pending = input_state(&mut s);
    assert!(
        pending.contains(r#""held":["KeyA","KeyZ"],"forwarded":["KeyA","KeyM"]"#),
        "{pending}"
    );
    assert!(
        pending.contains(r#""controls":[],"forwardedControls":["act"]"#),
        "{pending}"
    );
    assert!(pending.contains(r#""id":7,"action":"act""#), "{pending}");
    assert!(!pending.contains(r#""id":8"#), "{pending}");
    s.input(InputEvent::Control {
        name: "missing".into(),
        id: 9,
        phase: PointerPhase::Down,
        x: 0.,
        y: 0.,
        at_ms: 27.,
        player: 0,
    });
    assert_eq!(input_state(&mut s), pending);
    let mut restored = sim();
    restored.restore(&s.save().unwrap()).unwrap();
    assert_eq!(input_state(&mut restored), pending);
    s.input(InputEvent::Blur {
        at_ms: 30.,
        player: 0,
    });
    let blurred = input_state(&mut s);
    assert!(
        blurred.contains(r#""held":["KeyA","KeyZ"],"forwarded":[]"#),
        "{blurred}"
    );
    assert!(
        blurred.contains(r#""forwardedControls":[],"controlContacts":[]"#),
        "{blurred}"
    );
    s.advance(100., Clock::Seekable);
    s.bind(&CounterArgs { paused: true }.values(), None)
        .unwrap();
    key(&mut s, "KeyB", true, 40.);
    let paused = input_state(&mut s);
    assert!(
        paused.contains(r#""held":["KeyB"],"forwarded":["KeyB"]"#),
        "{paused}"
    );
    s.bind(&CounterArgs::default().values(), None).unwrap();
    assert_eq!(input_state(&mut s), paused);
}

#[test]
fn delivery_is_saved_but_never_changes_simulation_hash() {
    let mut a = sim();
    let before = a.world().hash();
    let epoch = a.world().mutation_epoch();
    a.world().emit("first");
    a.world().emit("second");
    assert_eq!(before, a.world().hash());
    assert_eq!(epoch, a.world().mutation_epoch());
    let saved = a.save().unwrap();
    let mut b = sim();
    b.restore(&saved).unwrap();
    assert_eq!(b.take_messages(), ["first", "second"]);
    assert_eq!(a.world().hash(), b.world().hash());
    a.run(100.0);
    b.run(100.0);
    assert_eq!(a.world().hash(), b.world().hash()); // observation-prefix hash path too
    let epoch = a.world().mutation_epoch();
    assert_eq!(a.take_messages(), ["first", "second"]);
    assert_eq!(epoch, a.world().mutation_epoch());
    assert_eq!(a.world().hash(), b.world().hash());
}

#[test]
fn restore_touch_viewport_continues_headless_and_resize_replaces_it() {
    let mut a = sim();
    a.viewport(800.0, 600.0);
    a.input(InputEvent::Pointer {
        id: 7,
        phase: PointerPhase::Down,
        x: 700.0,
        y: 300.0,
        dx: 0.0,
        dy: 0.0,
        buttons: 0,
        at_ms: 0.0,
        player: 0,
    });
    a.run(100.0);
    let mut b = sim();
    b.restore(&a.save().unwrap()).unwrap();
    a.run(100.0);
    b.run(100.0);
    assert_eq!(a.world().hash(), b.world().hash());
    let before = b.world().resource::<Counts>().held;
    b.viewport(1600.0, 600.0); // contact now lies in the left half
    b.run(100.0);
    assert_eq!(b.world().resource::<Counts>().held, before);
}

#[test]
fn old_save_containers_are_refused_by_name_atomically() {
    let mut s = sim();
    let saved = s.save().unwrap();
    let mut old = saved.clone();
    assert!(saved.starts_with(b"EXSIM\0\x08"));
    old[6] = 4;
    assert!(s
        .restore(&old)
        .unwrap_err()
        .to_string()
        .contains("EXSIM v8"));
    assert_eq!(s.save().unwrap(), saved);
    let saved = s.world().save();
    let mut old = saved.clone();
    assert!(saved.starts_with(b"EXGAME\0\x06"));
    old[7] = 3;
    assert!(s
        .world_mut()
        .load(&old)
        .unwrap_err()
        .to_string()
        .contains("EXGAME v6"));
    assert_eq!(s.world().save(), saved);
}

/// A v7 save (before gamepads and the input player) is refused by name.
#[test]
fn a_v7_save_is_refused_by_name() {
    let mut s = sim();
    let saved = s.save().unwrap();
    let mut old = saved.clone();
    old[6] = 7;
    let error = s.restore(&old).unwrap_err().to_string();
    assert!(error.contains("EXSIM v7 save predates v8"), "{error}");
    assert_eq!(s.save().unwrap(), saved);
}
#[test]
fn wrong_magic_reports_actual_bytes_and_expected_format() {
    let mut s = sim();
    let saved = s.save().unwrap();
    for bytes in [
        b"random!!".to_vec(),
        b"EXGAME\0\x03".to_vec(),
        b"EXSIM\0\x04!".to_vec(),
        vec![0xff; 8],
        b"short".to_vec(),
    ] {
        let seen = format!("{:02x?}", &bytes[..bytes.len().min(8)]);
        let error = s.restore(&bytes).unwrap_err().to_string();
        assert!(
            error.contains(&seen) && error.contains("EXSIM v8"),
            "{error}"
        );
        let error = s.world_mut().load(&bytes).unwrap_err().to_string();
        assert!(
            error.contains(&seen) && error.contains("EXGAME v6"),
            "{error}"
        );
        assert_eq!(s.save().unwrap(), saved);
    }
}

#[path = "sim/clock.rs"]
mod clock;

#[test]
fn restore_runs_no_setup_and_failed_world_validation_is_atomic() {
    thread_local! {
        static SETUPS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }
    struct Once;
    impl Game for Once {
        const ID: &'static str = "single-setup";
        type Args = ();
        fn setup(w: &mut World, _: &()) {
            SETUPS.with(|calls| calls.set(calls.get() + 1));
            w.spawn_named("player", Transform::default());
            w.emit("setup message");
        }
        fn tick(_: &mut World, _: &Input, _: &()) {}
    }
    let mut s = Sim::<Once>::new(()).unwrap().paranoid(Paranoid::Off);
    assert_eq!(SETUPS.with(|calls| calls.replace(0)), 1);
    let saved = s.save().unwrap();
    for bound in [false, true] {
        if bound {
            s.restore_bound(&saved).unwrap();
        } else {
            s.restore(&saved).unwrap();
        }
        assert_eq!(SETUPS.with(|calls| calls.replace(0)), 0);
        assert_eq!(s.save().unwrap(), saved);
    }
    // Invalid world headers must be rejected before setup has any side effects.
    let mut bad = saved.clone();
    let world = bad.windows(8).position(|v| v == b"EXGAME\0\x06").unwrap();
    bad[world] = b'!';
    assert!(s.restore(&bad).is_err());
    assert_eq!(SETUPS.with(|calls| calls.replace(0)), 0);
    assert_eq!(s.save().unwrap(), saved);
    // Game::register extends the scratch schema for the chosen arguments;
    // malformed typed data is refused before gameplay setup has side effects.
    let mut bad = saved.clone();
    bad[world + 8] = 0xff;
    assert!(s.restore(&bad).is_err());
    assert_eq!(SETUPS.with(|calls| calls.replace(0)), 0);
    assert_eq!(s.save().unwrap(), saved);
    for mode in [Paranoid::Save, Paranoid::FreshGame] {
        s = s.paranoid(mode);
        s.run(17.0);
        assert_eq!(SETUPS.with(|calls| calls.replace(0)), 0, "{mode:?}");
    }
}

#[test]
fn r13_empty_and_short_calls_take_all_remaining_rust_defaults() {
    #[derive(Args)]
    struct Options {
        seed: u64,
        #[live]
        paused: bool,
        #[restart]
        restart: bool,
    }
    impl Default for Options {
        fn default() -> Self {
            Self {
                seed: 17,
                paused: true,
                restart: false,
            }
        }
    }
    struct Defaults;
    impl Game for Defaults {
        const ID: &'static str = "r13-defaults";
        type Args = Options;
        fn setup(w: &mut World, args: &Options) {
            w.reseed(args.seed);
        }
        fn tick(_: &mut World, _: &Input, _: &Options) {}
    }
    let mut empty = Sim::<Defaults>::from_values(&[]).unwrap();
    assert_eq!(
        empty.save().unwrap(),
        Sim::<Defaults>::new(Options::default())
            .unwrap()
            .save()
            .unwrap()
    );
    let short = Sim::<Defaults>::from_values(&[Value::Number(7.)]).unwrap();
    let full =
        Sim::<Defaults>::from_values(&[Value::Number(7.), Value::Bool(true), Value::Bool(false)])
            .unwrap();
    assert_eq!(short.save().unwrap(), full.save().unwrap());
    empty.bind(&[Value::Number(7.)], None).unwrap();
    assert_eq!(empty.world().hash(), full.world().hash());
    assert!(empty
        .agent(r#"{"op":"state"}"#)
        .contains(r#""paused":true"#));
    assert!(Sim::<Defaults>::from_values(&[
        Value::Number(7.),
        Value::Bool(true),
        Value::Bool(false),
        Value::Bool(false)
    ])
    .err()
    .unwrap()
    .contains("got 4"));
}

#[test]
fn restore_registers_argument_dependent_types_before_setup() {
    thread_local! { static SETUPS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
    #[derive(Default, Args)]
    struct Options {
        other: bool,
    }
    #[derive(Default, exact_game::Component)]
    struct Other {
        value: u32,
    }
    struct Conditional;
    impl Game for Conditional {
        const ID: &'static str = "conditional-registration";
        type Args = Options;
        fn register(w: &mut World, args: &std::collections::BTreeMap<&str, Value>) {
            if args["other"].as_bool() == Some(true) {
                w.register::<Other>();
            }
        }
        fn setup(w: &mut World, args: &Options) {
            SETUPS.with(|calls| calls.set(calls.get() + 1));
            if args.other {
                w.spawn(Other { value: 17 });
            }
        }
        fn tick(_: &mut World, _: &Input, _: &Options) {}
    }
    let source = Sim::<Conditional>::new(Options { other: true }).unwrap();
    let mut target = Sim::<Conditional>::new(Options::default()).unwrap();
    target.restore(&source.save().unwrap()).unwrap();
    assert_eq!(source.world().hash(), target.world().hash());
    SETUPS.with(|calls| calls.set(0));
    let before = target.save().unwrap();
    let mut bad = before.clone();
    let world = bad.windows(8).position(|v| v == b"EXGAME\0\x06").unwrap();
    bad[world + 8] = 0xff;
    assert!(target.restore(&bad).is_err());
    assert_eq!(SETUPS.with(|calls| calls.get()), 0);
    assert_eq!(target.save().unwrap(), before);
}

#[test]
fn registration_filters_live_arguments_and_validates_before_register() {
    thread_local! { static REGISTERS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
    #[derive(Default, Args)]
    struct Options {
        extra: bool,
        #[live]
        enabled: bool,
    }
    #[derive(Default, exact_game::Component)]
    struct Extra(u32);
    struct Conditional;
    impl Game for Conditional {
        const ID: &'static str = "filtered-registration";
        type Args = Options;
        fn validate(a: &Options) -> Result<(), String> {
            if !a.extra {
                Err("extra required".into())
            } else {
                Ok(())
            }
        }
        fn register(w: &mut World, args: &std::collections::BTreeMap<&str, Value>) {
            REGISTERS.with(|n| n.set(n.get() + 1));
            assert!(!args.contains_key("enabled"));
            if args["extra"].as_bool() == Some(true) {
                w.register::<Extra>();
            }
        }
        fn setup(w: &mut World, _: &Options) {
            w.spawn(Extra(17));
        }
        fn tick(_: &mut World, _: &Input, _: &Options) {}
    }
    let mut source = Sim::<Conditional>::new(Options {
        extra: true,
        enabled: true,
    })
    .unwrap();
    source
        .bind(&[Value::Bool(true), Value::Bool(false)], None)
        .unwrap();
    let saved = source.save().unwrap();
    let mut target = Sim::<Conditional>::new(Options {
        extra: true,
        enabled: true,
    })
    .unwrap();
    target.restore(&saved).unwrap();
    assert_eq!(target.save().unwrap(), saved);
    REGISTERS.with(|n| n.set(0));
    assert!(Sim::<Conditional>::new(Options::default()).is_err());
    assert_eq!(REGISTERS.with(|n| n.get()), 0);
}

#[test]
fn saves_canonicalize_consumed_input_edges_and_preserve_pending_events() {
    let drive = |mode, epoch| {
        let mut s = Sim::<Counter>::new(CounterArgs::default())
            .unwrap()
            .paranoid(mode);
        s.viewport(800., 600.);
        s.advance(epoch, Clock::Seekable);
        s.input(InputEvent::Key {
            code: "KeyE".into(),
            down: true,
            at_ms: epoch + 5.,
            player: 0,
        });
        s.input(InputEvent::Pointer {
            id: 7,
            phase: PointerPhase::Down,
            x: 100.,
            y: 100.,
            dx: 0.,
            dy: 0.,
            buttons: 1,
            at_ms: epoch + 6.,
            player: 0,
        });
        s.input(InputEvent::Pointer {
            id: 7,
            phase: PointerPhase::Move,
            x: 110.,
            y: 105.,
            dx: 10.,
            dy: 5.,
            buttons: 1,
            at_ms: epoch + 7.,
            player: 0,
        });
        s.input(InputEvent::Wheel {
            dx: 2.,
            dy: -3.,
            at_ms: epoch + 8.,
            player: 0,
        });
        s.input(InputEvent::Key {
            code: "KeyE".into(),
            down: false,
            at_ms: epoch + 24.,
            player: 0,
        });
        s.input(InputEvent::Control {
            name: "act".into(),
            id: 9,
            phase: PointerPhase::Down,
            x: 0.,
            y: 0.,
            at_ms: epoch + 40.,
            player: 0,
        });
        s.input(InputEvent::Control {
            name: "act".into(),
            id: 9,
            phase: PointerPhase::Up,
            x: 0.,
            y: 0.,
            at_ms: epoch + 70.,
            player: 0,
        });
        s.advance(epoch + 17., Clock::Seekable);
        let saved = s.save().unwrap();
        assert_eq!(s.save().unwrap(), saved, "save is a read");
        let mut restored = sim().paranoid(mode);
        restored.restore(&saved).unwrap();
        assert_eq!(restored.save().unwrap(), saved, "{mode:?} edge boundary");
        s.advance(epoch + 100., Clock::Seekable);
        restored.advance(500_000., Clock::Seekable);
        restored.advance(500_083., Clock::Seekable);
        assert_eq!(restored.save().unwrap(), s.save().unwrap());
        let counts = s.world().resource::<Counts>();
        assert_eq!(counts.pressed, 2);
        assert_eq!(counts.released, 2);
        assert_eq!(counts.wheel, Vec2::new(2., -3.));
        (saved, s.save().unwrap())
    };
    for epoch in [0., 100_000.] {
        let expected = drive(Paranoid::Off, epoch);
        for mode in [Paranoid::Save, Paranoid::FreshGame] {
            assert_eq!(drive(mode, epoch), expected, "{mode:?}");
        }
    }
}

// Grow a Garden's HUD sent commands through a live argument: two presses between
// ticks kept only the second, and saves needed an acknowledge-and-reset protocol.
mod posted {
    use exact_game::*;
    #[derive(Args, Default)]
    pub struct Options {
        #[live]
        pub paused: bool,
    }
    #[derive(Default, Resource)]
    pub struct Log(pub Vec<(u64, String)>);
    pub struct Shop;
    impl Game for Shop {
        const ID: &'static str = "posted";
        type Args = Options;
        fn setup(w: &mut World, _: &Options) {
            w.insert_resource(Log::default());
        }
        fn paused(a: &Options) -> bool {
            a.paused
        }
        fn tick(w: &mut World, input: &Input, _: &Options) {
            for m in input.messages() {
                let tick = w.tick();
                w.resource_mut::<Log>().0.push((tick, m.clone()));
            }
        }
    }
}
#[test]
fn posted_messages_reach_one_tick_each_in_order_and_are_saved_only_while_pending() {
    use posted::*;
    let log = |s: &Sim<Shop>| s.world().resource::<Log>().0.clone();
    let mut s = Sim::<Shop>::new(Options::default()).unwrap();
    s.run(0.);
    s.post("buy carrot");
    s.post("buy carrot");
    s.post("sell all");
    let pending = s.save().unwrap();
    s.run(1000. / 60.);
    let delivered = vec![
        (0, "buy carrot".to_string()),
        (0, "buy carrot".to_string()),
        (0, "sell all".to_string()),
    ];
    assert_eq!(log(&s), delivered);
    s.run(1000.);
    assert_eq!(log(&s), delivered, "delivered once");
    // A pending message travels with a save; a delivered one does not.
    let mut fresh = Sim::<Shop>::new(Options::default()).unwrap();
    fresh.restore(&pending).unwrap();
    fresh.run(1000. / 60.);
    assert_eq!(log(&fresh), delivered);
    let after = s.save().unwrap();
    let mut again = Sim::<Shop>::new(Options::default()).unwrap();
    again.restore(&after).unwrap();
    again.run(1000.);
    assert_eq!(log(&again), delivered);
    assert_eq!(again.world().hash(), {
        s.run(1000.);
        s.world().hash()
    });
    // Paused worlds hold messages for the first tick after the pause.
    let mut p = Sim::<Shop>::new(Options { paused: true }).unwrap();
    p.run(0.);
    p.post("expand");
    p.run(500.);
    assert!(log(&p).is_empty());
    p.bind(&[Value::Bool(false)], None).unwrap();
    p.run(1000. / 60.);
    assert_eq!(log(&p).len(), 1);
    assert_eq!(log(&p)[0].1, "expand");
    // A full input queue refuses new posts by name and never drops one queued.
    let mut full = Sim::<Shop>::new(Options { paused: true }).unwrap();
    full.run(0.);
    for i in 0..1100 {
        full.post(format!("m{i}"));
    }
    assert!(full
        .agent(r#"{"op":"state"}"#)
        .contains(r#""refusedPosts":76"#));
    full.bind(&[Value::Bool(false)], None).unwrap();
    full.run(1000. / 60.);
    let got: Vec<_> = log(&full).into_iter().map(|(_, m)| m).collect();
    assert_eq!(got, (0..1024).map(|i| format!("m{i}")).collect::<Vec<_>>());
    let refusals = full
        .world()
        .journal()
        .iter()
        .filter(|e| e.line.contains("postMessage refused"))
        .count();
    assert_eq!(refusals, 1, "one journal line per overflow episode");
    // The drained queue ends the episode: the next overflow journals again.
    full.bind(&[Value::Bool(true)], None).unwrap();
    for i in 0..1100 {
        full.post(format!("n{i}"));
    }
    let refusals = full
        .world()
        .journal()
        .iter()
        .filter(|e| e.line.contains("postMessage refused"))
        .count();
    assert_eq!(refusals, 2, "two separate overflow episodes give two lines");
    // Oversized messages refuse by name and change nothing.
    let mut big = Sim::<Shop>::new(Options::default()).unwrap();
    big.run(0.);
    big.post("x".repeat(70_000));
    big.run(100.);
    assert!(log(&big).is_empty());
    assert!(big
        .world()
        .journal()
        .iter()
        .any(|e| e.line.contains("exceeds")));
}
