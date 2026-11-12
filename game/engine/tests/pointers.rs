//! Every contact a tick can read (`Input::pointers`, `Input::pointer_events`),
//! each by its own id, while `Input::pointer` keeps its one primary pointer.
use exact_game::*;

#[derive(Clone, Debug, Default, Data)]
struct Seen {
    tick: u64,
    pointers: Vec<PointerContact>,
    edges: Vec<PointerEdge>,
    primary: Option<PointerState>,
}
#[derive(Default, Resource)]
struct Log(Vec<Seen>);
#[derive(Default, Args)]
pub struct Options {
    #[live]
    paused: bool,
}
struct Fingers;
impl Game for Fingers {
    const ID: &'static str = "pointers";
    const HZ: u32 = 10;
    type Args = Options;
    fn setup(w: &mut World, _: &Options) {
        w.insert_resource(Log::default());
    }
    fn paused(args: &Options) -> bool {
        args.paused
    }
    fn tick(w: &mut World, input: &Input, _: &Options) {
        let seen = Seen {
            tick: w.tick(),
            pointers: input.pointers().to_vec(),
            edges: input.pointer_events().to_vec(),
            primary: input.pointer(),
        };
        w.resource_mut::<Log>().0.push(seen);
    }
}
const TICK: f64 = 100.;
fn sim() -> Sim<Fingers> {
    let mut s = Sim::new(Options::default()).unwrap();
    s.advance(0., Clock::Seekable);
    s
}
fn event(id: u64, phase: PointerPhase, x: f32, buttons: u32, at_ms: f64) -> InputEvent {
    InputEvent::Pointer {
        id,
        phase,
        x,
        y: 5.,
        dx: if phase == PointerPhase::Move { 1. } else { 0. },
        dy: 0.,
        buttons,
        at_ms,
        player: 0,
    }
}
fn touch(s: &mut Sim<Fingers>, id: u64, phase: PointerPhase, x: f32, at_ms: f64) {
    s.input(event(id, phase, x, 0, at_ms));
}
/// Run to the end of tick `n` and return what that tick read.
fn tick(s: &mut Sim<Fingers>, n: u64) -> Seen {
    s.advance(n as f64 * TICK, Clock::Seekable);
    s.world().resource::<Log>().0.last().unwrap().clone()
}
fn edges(seen: &Seen) -> Vec<(u64, PointerPhase)> {
    seen.edges.iter().map(|e| (e.id, e.phase)).collect()
}
fn at(x: f32) -> Vec2 {
    Vec2::new(x, 5.)
}
fn contact(id: u64, origin: f32, x: f32, down: bool, delta: f32) -> PointerContact {
    PointerContact {
        id,
        position: at(x),
        press_origin: at(origin),
        down,
        delta: Vec2::new(delta, 0.),
    }
}
/// Touch's own tap rule: a Down and its Up on one id, no Cancel between.
fn taps(log: &[Seen]) -> Vec<u64> {
    let mut down = Vec::new();
    let mut taps = Vec::new();
    for e in log.iter().flat_map(|s| &s.edges) {
        match e.phase {
            PointerPhase::Down => down.push(e.id),
            PointerPhase::Up if down.contains(&e.id) => {
                down.retain(|id| *id != e.id);
                taps.push(e.id);
            }
            _ => down.retain(|id| *id != e.id),
        }
    }
    taps
}

#[test]
fn two_downs_in_one_tick_are_both_visible() {
    let mut s = sim();
    touch(&mut s, 9, PointerPhase::Down, 90., 10.);
    touch(&mut s, 4, PointerPhase::Down, 40., 20.);
    let seen = tick(&mut s, 1);
    assert_eq!(
        seen.pointers,
        [
            contact(4, 40., 40., true, 0.),
            contact(9, 90., 90., true, 0.)
        ]
    );
    assert_eq!(
        edges(&seen),
        [(9, PointerPhase::Down), (4, PointerPhase::Down)]
    );
    assert_eq!(seen.edges[0].world_us, 10_000);
    assert_eq!(seen.edges[1].position, at(40.));
    // The primary pointer is still the first one down.
    assert_eq!(seen.primary.unwrap().id, 9);
}
#[test]
fn moving_b_leaves_a_unchanged() {
    let mut s = sim();
    touch(&mut s, 1, PointerPhase::Down, 10., 10.);
    touch(&mut s, 2, PointerPhase::Down, 20., 20.);
    tick(&mut s, 1);
    touch(&mut s, 2, PointerPhase::Move, 25., 110.);
    touch(&mut s, 2, PointerPhase::Move, 30., 120.);
    let seen = tick(&mut s, 2);
    assert_eq!(
        seen.pointers,
        [
            contact(1, 10., 10., true, 0.),
            contact(2, 20., 30., true, 2.)
        ]
    );
    assert!(seen.edges.is_empty());
    let seen = tick(&mut s, 3);
    assert_eq!(seen.pointers[1].delta, Vec2::ZERO, "a delta lasts one tick");
}
#[test]
fn lifting_a_keeps_its_up_edge_while_b_stays_down() {
    let mut s = sim();
    touch(&mut s, 1, PointerPhase::Down, 10., 10.);
    touch(&mut s, 2, PointerPhase::Down, 20., 20.);
    tick(&mut s, 1);
    touch(&mut s, 1, PointerPhase::Up, 12., 110.);
    let seen = tick(&mut s, 2);
    assert_eq!(edges(&seen), [(1, PointerPhase::Up)]);
    assert_eq!(seen.edges[0].position, at(12.));
    assert_eq!(
        seen.pointers,
        [
            contact(1, 10., 12., false, 0.),
            contact(2, 20., 20., true, 0.)
        ]
    );
    let seen = tick(&mut s, 3);
    assert_eq!(seen.pointers, [contact(2, 20., 20., true, 0.)]);
    assert!(seen.edges.is_empty());
}
#[test]
fn a_cancelled_contact_never_taps() {
    let mut s = sim();
    touch(&mut s, 3, PointerPhase::Down, 10., 10.);
    tick(&mut s, 1);
    touch(&mut s, 3, PointerPhase::Cancel, 10., 110.);
    // An Up after the cancel is not the contact's: it ended.
    touch(&mut s, 3, PointerPhase::Up, 10., 120.);
    let seen = tick(&mut s, 2);
    assert_eq!(edges(&seen), [(3, PointerPhase::Cancel)]);
    assert!(!seen.pointers[0].down);
    tick(&mut s, 3);
    assert!(taps(&s.world().resource::<Log>().0).is_empty());
}
#[test]
fn a_down_and_up_in_one_tick_are_both_visible() {
    let mut s = sim();
    touch(&mut s, 5, PointerPhase::Down, 10., 10.);
    touch(&mut s, 5, PointerPhase::Move, 14., 20.);
    touch(&mut s, 5, PointerPhase::Up, 15., 30.);
    let seen = tick(&mut s, 1);
    assert_eq!(
        edges(&seen),
        [(5, PointerPhase::Down), (5, PointerPhase::Up)]
    );
    assert_eq!(
        (seen.edges[0].position, seen.edges[1].position),
        (at(10.), at(15.))
    );
    assert_eq!(seen.pointers, [contact(5, 10., 15., false, 1.)]);
    assert!(tick(&mut s, 2).pointers.is_empty());
}
#[test]
fn two_full_tap_cycles_keep_their_order() {
    let mut s = sim();
    touch(&mut s, 1, PointerPhase::Down, 10., 10.);
    touch(&mut s, 2, PointerPhase::Down, 20., 20.);
    touch(&mut s, 2, PointerPhase::Up, 20., 30.);
    touch(&mut s, 1, PointerPhase::Up, 10., 40.);
    let seen = tick(&mut s, 1);
    assert_eq!(
        edges(&seen),
        [
            (1, PointerPhase::Down),
            (2, PointerPhase::Down),
            (2, PointerPhase::Up),
            (1, PointerPhase::Up)
        ]
    );
    let stamps: Vec<_> = seen.edges.iter().map(|e| e.world_us).collect();
    assert_eq!(stamps, [10_000, 20_000, 30_000, 40_000]);
    assert_eq!(taps(&s.world().resource::<Log>().0), [2, 1]);
}
#[test]
fn blur_cancels_every_contact() {
    let mut s = sim();
    touch(&mut s, 1, PointerPhase::Down, 10., 10.);
    touch(&mut s, 2, PointerPhase::Down, 20., 20.);
    tick(&mut s, 1);
    touch(&mut s, 2, PointerPhase::Move, 25., 110.);
    s.input(InputEvent::Blur {
        at_ms: 120.,
        player: 0,
    });
    let seen = tick(&mut s, 2);
    assert_eq!(
        edges(&seen),
        [(1, PointerPhase::Cancel), (2, PointerPhase::Cancel)]
    );
    assert_eq!(seen.edges[1].position, at(25.));
    assert!(seen.edges.iter().all(|e| e.world_us == 120_000));
    assert!(seen.pointers.iter().all(|p| !p.down));
    assert!(seen.primary.is_none());
    assert!(tick(&mut s, 3).pointers.is_empty());
    assert!(taps(&s.world().resource::<Log>().0).is_empty());
}
#[test]
fn an_id_is_reusable_after_release() {
    let mut s = sim();
    touch(&mut s, 7, PointerPhase::Down, 10., 10.);
    touch(&mut s, 7, PointerPhase::Up, 10., 20.);
    touch(&mut s, 7, PointerPhase::Down, 50., 30.);
    let seen = tick(&mut s, 1);
    assert_eq!(
        edges(&seen),
        [
            (7, PointerPhase::Down),
            (7, PointerPhase::Up),
            (7, PointerPhase::Down)
        ]
    );
    assert_eq!(seen.pointers, [contact(7, 50., 50., true, 0.)]);
    touch(&mut s, 7, PointerPhase::Up, 55., 110.);
    tick(&mut s, 2);
    touch(&mut s, 7, PointerPhase::Down, 80., 210.);
    let seen = tick(&mut s, 3);
    assert_eq!(edges(&seen), [(7, PointerPhase::Down)]);
    assert_eq!(seen.pointers, [contact(7, 80., 80., true, 0.)]);
    assert_eq!(taps(&s.world().resource::<Log>().0), [7, 7]);
}
#[test]
fn save_and_restore_keep_active_origins_without_stale_edges() {
    let mut s = sim();
    touch(&mut s, 1, PointerPhase::Down, 10., 10.);
    touch(&mut s, 2, PointerPhase::Down, 20., 20.);
    touch(&mut s, 2, PointerPhase::Move, 30., 30.);
    touch(&mut s, 3, PointerPhase::Down, 40., 40.);
    touch(&mut s, 3, PointerPhase::Up, 40., 50.);
    let live = tick(&mut s, 1);
    assert_eq!(live.edges.len(), 4);
    let saved = s.save().unwrap();
    let mut restored = Sim::<Fingers>::new(Options::default()).unwrap();
    restored.advance(5000., Clock::Seekable);
    restored.restore(&saved).unwrap();
    assert_eq!(restored.save().unwrap(), saved);
    // The restored game's first advance sets its new epoch and runs no tick.
    restored.advance(5000. + TICK, Clock::Seekable);
    let mut tails = Vec::new();
    for (game, epoch) in [(&mut s, 0.), (&mut restored, 5000.)] {
        let before = game.world().resource::<Log>().0.len();
        game.advance(epoch + 2. * TICK, Clock::Seekable);
        touch(game, 2, PointerPhase::Up, 35., epoch + 2. * TICK + 10.);
        game.advance(epoch + 3. * TICK, Clock::Seekable);
        let log = game.world().resource::<Log>().0[before..].to_vec();
        let [held, last] = &log[..] else {
            panic!("two ticks ran, not {}", log.len());
        };
        assert!(held.edges.is_empty(), "no edge is replayed");
        assert_eq!(
            held.pointers,
            [
                contact(1, 10., 10., true, 0.),
                contact(2, 20., 30., true, 0.)
            ]
        );
        assert_eq!(edges(last), [(2, PointerPhase::Up)]);
        assert_eq!(last.edges[0].world_us, 210_000);
        assert_eq!(last.pointers[1], contact(2, 20., 35., false, 0.));
        tails.push(
            log.iter()
                .map(|t| (t.tick, t.pointers.clone(), t.edges.clone()))
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(tails[0], tails[1]);
    assert_eq!(s.save().unwrap(), restored.save().unwrap());
}
#[test]
fn paused_input_causes_no_delayed_tap() {
    let mut s = sim();
    tick(&mut s, 1);
    s.bind(&[Value::Bool(true)], None).unwrap();
    touch(&mut s, 1, PointerPhase::Down, 10., 110.);
    touch(&mut s, 1, PointerPhase::Up, 10., 120.);
    touch(&mut s, 2, PointerPhase::Down, 20., 130.);
    s.advance(300., Clock::Seekable);
    s.bind(&[Value::Bool(false)], None).unwrap();
    let seen = tick(&mut s, 4);
    assert!(seen.edges.is_empty());
    // The contact still down when the pause ended is held, not pressed.
    assert_eq!(seen.pointers, [contact(2, 20., 20., true, 0.)]);
    touch(&mut s, 2, PointerPhase::Up, 20., 410.);
    tick(&mut s, 5);
    assert!(taps(&s.world().resource::<Log>().0).is_empty());
}
#[test]
fn mouse_hover_and_secondary_buttons_keep_the_primary_pointer() {
    let mut s = sim();
    // Hover is the primary pointer and no contact.
    s.input(event(1, PointerPhase::Move, 10., 0, 10.));
    let seen = tick(&mut s, 1);
    assert!(seen.pointers.is_empty() && seen.edges.is_empty());
    let hover = seen.primary.unwrap();
    assert!(!hover.down && hover.press_origin.is_none());
    // A secondary press is a contact; it sets no primary press origin.
    s.input(event(1, PointerPhase::Down, 20., 2, 110.));
    let seen = tick(&mut s, 2);
    assert_eq!(edges(&seen), [(1, PointerPhase::Down)]);
    let p = seen.primary.unwrap();
    assert!(p.down && p.press_origin.is_none());
    // MouseLeft added to the held button: the primary origin moves, the
    // contact continues.
    s.input(event(1, PointerPhase::Down, 30., 3, 210.));
    // Releasing one button of two keeps the contact down.
    s.input(event(1, PointerPhase::Up, 35., 1, 220.));
    let seen = tick(&mut s, 3);
    assert!(seen.edges.is_empty());
    assert_eq!(seen.pointers, [contact(1, 30., 35., true, 0.)]);
    assert_eq!(seen.primary.unwrap().press_origin, Some(at(30.)));
    s.input(event(1, PointerPhase::Up, 40., 0, 310.));
    let seen = tick(&mut s, 4);
    assert_eq!(edges(&seen), [(1, PointerPhase::Up)]);
    let p = seen.primary.unwrap();
    assert!(!p.down && p.press_origin == Some(at(30.)) && p.position == at(40.));
    // A touch while the mouse hovers replaces neither in the other's API.
    s.input(event(1, PointerPhase::Move, 45., 0, 410.));
    touch(&mut s, 8, PointerPhase::Down, 60., 420.);
    let seen = tick(&mut s, 5);
    assert_eq!(seen.pointers, [contact(8, 60., 60., true, 0.)]);
    assert_eq!(seen.primary.unwrap().id, 8, "a Down replaces a hover");
}

/// The game guide's multi-finger grab (game/README.md, Controls), with a
/// die as a named entity on the x axis and a pick of the nearest within 10 points.
mod grab {
    use exact_game::*;
    #[derive(Clone, Default, Data)]
    pub struct Grab {
        finger: u64,
        die: Entity,
    }
    #[derive(Default, Resource)]
    pub struct Grabs(pub Vec<Grab>);
    pub struct Dice;
    fn die_under(w: &World, _viewport: Vec2, at: Vec2) -> Option<Entity> {
        ["a", "b", "c"]
            .into_iter()
            .filter_map(|name| w.named(name))
            .find(|&e| (w.get::<Transform>(e).unwrap().position.x - at.x).abs() < 10.)
    }
    fn drag_to(w: &mut World, die: Entity, _viewport: Vec2, at: Vec2) {
        w.get_mut::<Transform>(die).unwrap().position.x = at.x;
    }
    impl Game for Dice {
        const ID: &'static str = "grab";
        const HZ: u32 = 10;
        type Args = super::Options;
        fn setup(w: &mut World, _: &super::Options) {
            w.insert_resource(Grabs::default());
            for (name, x) in [("a", 0.), ("b", 100.), ("c", 200.)] {
                w.spawn_named(name, (Transform::at(x, 0., 0.),));
            }
        }
        fn paused(args: &super::Options) -> bool {
            args.paused
        }
        fn tick(w: &mut World, input: &Input, _: &super::Options) {
            for e in input.pointer_events() {
                w.resource_mut::<Grabs>().0.retain(|g| g.finger != e.id);
                if e.phase == PointerPhase::Down {
                    if let Some(die) = die_under(w, input.viewport(), e.position) {
                        w.resource_mut::<Grabs>().0.push(Grab { finger: e.id, die });
                    }
                }
            }
            // A finger no longer down lets go, though it lifted during a pause.
            let down = |id| input.pointers().iter().any(|p| p.id == id && p.down);
            w.resource_mut::<Grabs>().0.retain(|g| down(g.finger));
            let grabs = w.resource::<Grabs>().0.clone();
            for p in input.pointers() {
                if let Some(g) = grabs.iter().find(|g| g.finger == p.id) {
                    drag_to(w, g.die, input.viewport(), p.position);
                }
            }
        }
    }
}
#[test]
fn each_finger_holds_the_die_under_it() {
    let mut s = Sim::<grab::Dice>::new(Options::default()).unwrap();
    s.advance(0., Clock::Seekable);
    let x = |s: &Sim<grab::Dice>, name: &str| s.get::<Transform>(name).unwrap().position.x;
    s.input(event(1, PointerPhase::Down, 2., 0, 10.));
    s.input(event(2, PointerPhase::Down, 198., 0, 20.));
    s.advance(100., Clock::Seekable);
    s.input(event(1, PointerPhase::Move, 40., 0, 110.));
    s.input(event(2, PointerPhase::Move, 260., 0, 120.));
    s.advance(200., Clock::Seekable);
    assert_eq!((x(&s, "a"), x(&s, "b"), x(&s, "c")), (40., 100., 260.));
    s.input(event(1, PointerPhase::Up, 40., 0, 210.));
    s.input(event(2, PointerPhase::Move, 300., 0, 220.));
    s.input(event(1, PointerPhase::Down, 95., 0, 230.));
    s.input(event(1, PointerPhase::Move, 120., 0, 240.));
    s.advance(300., Clock::Seekable);
    assert_eq!((x(&s, "a"), x(&s, "b"), x(&s, "c")), (40., 120., 300.));
    // Finger 1 lifts and its id lands again on no die: it holds nothing.
    s.input(event(1, PointerPhase::Up, 120., 0, 310.));
    s.input(event(1, PointerPhase::Down, 160., 0, 320.));
    s.input(event(1, PointerPhase::Move, 170., 0, 330.));
    s.advance(400., Clock::Seekable);
    assert_eq!(x(&s, "b"), 120.);
    s.input(event(1, PointerPhase::Up, 170., 0, 410.));
    s.input(event(1, PointerPhase::Down, 118., 0, 420.));
    s.advance(500., Clock::Seekable);
    assert_eq!(s.world().resource::<grab::Grabs>().0.len(), 2);
    // Both fingers lift while the game is paused: neither holds a die after.
    s.bind(&[Value::Bool(true)], None).unwrap();
    s.input(event(1, PointerPhase::Up, 118., 0, 510.));
    s.input(InputEvent::Blur {
        at_ms: 520.,
        player: 0,
    });
    s.advance(600., Clock::Seekable);
    s.bind(&[Value::Bool(false)], None).unwrap();
    s.advance(700., Clock::Seekable);
    assert!(s.world().resource::<grab::Grabs>().0.is_empty());
}
