//! Opt-in, single-finger vertical scrolling for the KMS kiosk probe.
//! The presenter owns logical scrolling/virtualization; this shell owns touch physics.
use crate::input::InputEvent;
use exact_painted::Presenter;
use exact_runner::DataSource;

// Rendering may queue several evdev reports. Sample the final position once per
// batch, rather than treating dispatch microseconds as the finger's travel time.
pub(super) fn coalesce(events: Vec<InputEvent>) -> Vec<InputEvent> {
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        if let InputEvent::Touch(x, y) = event {
            if let Some(InputEvent::Touch(last_x, last_y)) = out.last_mut() {
                *last_x = x.or(*last_x);
                *last_y = y.or(*last_y);
                continue;
            }
        }
        out.push(event);
    }
    out
}

#[derive(Clone, Copy)]
enum Phase {
    Pending { origin: (f32, f32) },
    Dragging,
    Coasting,
}
struct Gesture {
    id: u32,
    host: u64,
    phase: Phase,
    position: f32,
    velocity: f32,
    last: (f32, f32),
    moved_at: f64,
    tick_at: f64,
    height: f32,
}
#[derive(Default)]
pub(super) struct TouchScroll {
    gesture: Option<Gesture>,
}
fn drag(mut position: f32, mut delta: f32, max: f32, height: f32) -> f32 {
    if (0. ..=max).contains(&position) {
        let next = position + delta;
        position = next.clamp(0., max);
        delta = next - position;
    }
    let edge = position.clamp(0., max);
    let over = position - edge;
    let resistance = 0.55 * (1. - (over.abs() / height.max(1.)).min(1.)).powi(2);
    let next = over + delta * resistance;
    if over != 0. && next.signum() != over.signum() {
        (edge + delta + over / resistance).clamp(0., max)
    } else {
        edge + next
    }
}
fn coast(position: &mut f32, velocity: &mut f32, max: f32, dt: f32) {
    let bound = position.clamp(0., max);
    if *position != bound {
        // Closed-form critically damped spring; stable even when frames are late.
        let x = *position - bound;
        let w = 10.;
        let c = *velocity + w * x;
        let decay = (-w * dt).exp();
        *position = bound + (x + c * dt) * decay;
        *velocity = (*velocity - w * c * dt) * decay;
    } else {
        let decay = (-dt / 0.5).exp();
        *position += *velocity * 0.5 * (1. - decay);
        *velocity *= decay;
    }
}
impl TouchScroll {
    pub(super) fn animating(&self) -> bool {
        self.gesture
            .as_ref()
            .is_some_and(|g| matches!(g.phase, Phase::Coasting))
    }
    fn clear<D: DataSource>(&mut self, p: &mut Presenter<D>) {
        self.gesture = None;
        p.kiosk_clear_overscroll();
    }
    pub(super) fn input<D: DataSource>(
        &mut self,
        p: &mut Presenter<D>,
        pointer: &mut (f32, f32),
        viewport: (f32, f32),
        event: InputEvent,
        now: f64,
    ) -> bool {
        match event {
            InputEvent::TouchDown => {
                p.set_touching(true);
                let under = p.scrollers_at(pointer.0, pointer.1);
                let target = (!under.claims.1 && under.field.is_none())
                    .then(|| under.chain.into_iter().find(|s| s.id != 0 && s.axes.1))
                    .flatten();
                let Some(s) = target else {
                    self.clear(p);
                    return false;
                };
                let catch = self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.id == s.id && matches!(g.phase, Phase::Coasting));
                let position = s.offset.1
                    + p.kiosk_overscroll()
                        .filter(|(id, _)| *id == s.id)
                        .map_or(0., |(_, y)| y);
                self.gesture = Some(Gesture {
                    id: s.id,
                    host: p.hosts(),
                    phase: if catch {
                        Phase::Dragging
                    } else {
                        Phase::Pending { origin: *pointer }
                    },
                    position,
                    velocity: 0.,
                    last: *pointer,
                    moved_at: now,
                    tick_at: now,
                    height: s.port[3],
                });
                if catch {
                    let _ = p.pointer_cancel(now);
                }
                catch
            }
            InputEvent::Touch(x, y) => {
                let Some(g) = &mut self.gesture else {
                    return false;
                };
                if matches!(g.phase, Phase::Coasting) {
                    return false;
                }
                let next = (
                    x.map_or(pointer.0, |v| (v * viewport.0).clamp(0., viewport.0 - 1.)),
                    y.map_or(pointer.1, |v| (v * viewport.1).clamp(0., viewport.1 - 1.)),
                );
                let Some(s) = p.kiosk_scroll_state(g.id).filter(|_| g.host == p.hosts()) else {
                    self.clear(p);
                    return false;
                };
                if let Phase::Pending { origin } = g.phase {
                    let dx = next.0 - origin.0;
                    let dy = next.1 - origin.1;
                    if dy.abs() < 8. || dy.abs() <= dx.abs() {
                        g.last = next;
                        return false;
                    }
                    let _ = p.pointer_cancel(now);
                    g.phase = Phase::Dragging;
                }
                // evdev repeats the contact position before its release even when it stayed still.
                // Coasting starts with the velocity of the last movement.
                if next == g.last {
                    *pointer = next;
                    return true;
                }
                let before = g.position;
                g.position = drag(g.position, g.last.1 - next.1, s.max.1, g.height);
                let after = g.position;
                let dt = ((now - g.moved_at) / 1000.) as f32;
                if dt > 0. {
                    let sample = ((after - before) / dt).clamp(-8000., 8000.);
                    g.velocity = if dt > 0.08 {
                        sample
                    } else {
                        // Keep the smoothing time constant when rendering batches reports.
                        let decay = (-dt / 0.05).exp();
                        g.velocity * decay + sample * (1. - decay)
                    };
                }
                g.last = next;
                g.moved_at = now;
                *pointer = next;
                p.kiosk_scroll_to(
                    g.id,
                    g.position.clamp(0., s.max.1),
                    after - g.position.clamp(0., s.max.1),
                );
                true
            }
            InputEvent::TouchUp => {
                p.set_touching(false);
                let Some(g) = &mut self.gesture else {
                    return false;
                };
                if matches!(g.phase, Phase::Pending { .. }) {
                    self.clear(p);
                    return false;
                }
                // The shell consumes the lift after taking the drag from the presenter.
                p.pointer_released();
                let Some(_) = p.kiosk_scroll_state(g.id) else {
                    self.clear(p);
                    return true;
                };
                if now - g.moved_at > 80. {
                    g.velocity = 0.;
                }
                g.phase = Phase::Coasting;
                g.tick_at = now;
                true
            }
            InputEvent::Cancel | InputEvent::Wheel(..) | InputEvent::MouseDown(_) => {
                if matches!(event, InputEvent::Cancel) {
                    p.set_touching(false);
                }
                self.clear(p);
                false
            }
            _ => false,
        }
    }
    pub(super) fn tick<D: DataSource>(&mut self, p: &mut Presenter<D>, now: f64) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if !matches!(g.phase, Phase::Coasting) {
            return;
        }
        let Some(s) = p.kiosk_scroll_state(g.id).filter(|_| g.host == p.hosts()) else {
            self.clear(p);
            return;
        };
        let dt = (((now - g.tick_at) / 1000.) as f32).max(0.);
        g.tick_at = now;
        coast(&mut g.position, &mut g.velocity, s.max.1, dt);
        let bound = g.position.clamp(0., s.max.1);
        let decay_time = if g.position == bound { 0.5 } else { 0.1 };
        let settled = g.velocity.abs() * decay_time < 0.5 && (g.position - bound).abs() < 0.5;
        p.kiosk_scroll_to(g.id, bound, if settled { 0. } else { g.position - bound });
        if settled {
            self.clear(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exact_painted::presenter::PainterChoice;
    use std::path::PathBuf;

    fn fixture() -> (Presenter<()>, TouchScroll, (f32, f32), u32) {
        let plan = contract::compile(
            r#"component App
  view
    column width=400 height=500
      box height=100 flex-shrink=0
      column testId="list" width=400 height=400 flex-shrink=0 overflow-y="scroll"
        box width=400 height=2000 flex-shrink=0 background-color="red"
"#,
        )
        .unwrap();
        let mut p = Presenter::boot_with(
            &plan.encode(),
            (),
            (400., 500.),
            1.,
            PathBuf::from("."),
            PainterChoice::Cpu,
        )
        .unwrap()
        .0;
        let id = p
            .scrollers_at(200., 300.)
            .chain
            .into_iter()
            .find(|s| s.id != 0 && s.axes.1)
            .unwrap()
            .id;
        (p, TouchScroll::default(), (200., 300.), id)
    }
    fn input(
        s: &mut TouchScroll,
        p: &mut Presenter<()>,
        point: &mut (f32, f32),
        event: InputEvent,
        now: f64,
    ) -> bool {
        s.input(p, point, (400., 500.), event, now)
    }
    #[test]
    fn dragged_and_caught_touches_hold_reloads_until_the_finger_lifts_or_is_lost() {
        use exact_painted::presenter::ReloadResult;
        let next = contract::compile("component App\n  view\n    text \"next\"\n")
            .unwrap()
            .encode();
        for (catch, end) in [
            (false, InputEvent::TouchUp),
            (true, InputEvent::TouchUp),
            (true, InputEvent::Cancel),
        ] {
            let (mut p, scroll, at, _) = fixture();
            let mut pointer = super::super::pointer::Pointer::new(at);
            pointer.scroll = Some(scroll);
            let mut send = |p: &mut Presenter<()>, event, now| {
                super::super::pointer::dispatch(p, &mut pointer, (400., 500.), 1., event, now)
                    .unwrap();
            };
            send(&mut p, InputEvent::TouchDown, 0.);
            send(&mut p, InputEvent::Touch(None, Some(0.4)), 20.);
            if catch {
                send(&mut p, InputEvent::TouchUp, 21.);
                send(&mut p, InputEvent::TouchDown, 22.);
            }
            let before = p.hosts();
            assert_eq!(p.reload(&next, ()).unwrap(), ReloadResult::Held("pointer"));
            send(&mut p, end, 30.);
            p.run_commands(|| ());
            assert_eq!(p.hosts(), before + 1);
        }
    }
    #[test]
    fn kiosk_scroll_tap_drag_fling_and_catch() {
        let (mut p, mut s, mut point, id) = fixture();
        assert!(!input(
            &mut s,
            &mut p,
            &mut point,
            InputEvent::TouchDown,
            0.
        ));
        assert!(!input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 10.));
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, 0.);
        input(&mut s, &mut p, &mut point, InputEvent::TouchDown, 20.);
        assert!(input(
            &mut s,
            &mut p,
            &mut point,
            InputEvent::Touch(None, Some(0.4)),
            40.
        ));
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, 100.);
        assert!(input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 41.));
        s.tick(&mut p, 57.);
        let moving = p.kiosk_scroll_state(id).unwrap().offset.1;
        assert!(moving > 100.);
        assert!(input(
            &mut s,
            &mut p,
            &mut point,
            InputEvent::TouchDown,
            58.
        ));
        assert!(!s.animating());
        s.tick(&mut p, 100.);
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, moving);
    }
    #[test]
    fn kiosk_scroll_overscroll_keeps_logical_offset_and_returns() {
        let (mut p, mut s, mut point, id) = fixture();
        input(&mut s, &mut p, &mut point, InputEvent::TouchDown, 0.);
        input(
            &mut s,
            &mut p,
            &mut point,
            InputEvent::Touch(None, Some(0.8)),
            20.,
        );
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, 0.);
        let over = p.kiosk_overscroll().unwrap().1;
        assert!(over < 0. && over > -100.);
        input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 150.);
        // Catch and release a bounce without changing its visual position.
        input(&mut s, &mut p, &mut point, InputEvent::TouchDown, 151.);
        input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 152.);
        assert!((s.gesture.as_ref().unwrap().position - over).abs() < 0.001);
        for n in 1..180 {
            s.tick(&mut p, 152. + n as f64 * 16.);
        }
        assert!(!s.animating());
        assert!(p.kiosk_overscroll().is_none());
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, 0.);
    }
    #[test]
    fn kiosk_scroll_queued_motion_keeps_press_and_release_boundaries() {
        let events = coalesce(vec![
            InputEvent::Touch(Some(0.5), Some(0.8)),
            InputEvent::TouchDown,
            InputEvent::Touch(None, Some(0.7)),
            InputEvent::Touch(Some(0.6), None),
            InputEvent::Touch(None, Some(0.5)),
            InputEvent::TouchUp,
        ]);
        assert_eq!(
            events,
            vec![
                InputEvent::Touch(Some(0.5), Some(0.8)),
                InputEvent::TouchDown,
                InputEvent::Touch(Some(0.6), Some(0.5)),
                InputEvent::TouchUp
            ]
        );
    }
    #[test]
    fn kiosk_scroll_same_batch_does_not_invent_velocity() {
        let (mut p, mut s, mut point, id) = fixture();
        input(&mut s, &mut p, &mut point, InputEvent::TouchDown, 100.);
        input(
            &mut s,
            &mut p,
            &mut point,
            InputEvent::Touch(None, Some(0.4)),
            100.,
        );
        input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 100.);
        s.tick(&mut p, 116.);
        assert_eq!(p.kiosk_scroll_state(id).unwrap().offset.1, 100.);
        assert!(!s.animating());
    }
    #[test]
    fn kiosk_scroll_velocity_is_independent_of_report_frequency() {
        let mut velocities = Vec::new();
        for step in [10, 50] {
            let (mut p, mut s, mut point, _) = fixture();
            input(&mut s, &mut p, &mut point, InputEvent::TouchDown, 0.);
            for t in (step..=200).step_by(step) {
                input(
                    &mut s,
                    &mut p,
                    &mut point,
                    InputEvent::Touch(None, Some((300. - t as f32) / 500.)),
                    t as f64,
                );
            }
            let velocity = s.gesture.as_ref().unwrap().velocity;
            input(
                &mut s,
                &mut p,
                &mut point,
                InputEvent::Touch(Some(0.5), Some(0.2)),
                220.,
            );
            input(&mut s, &mut p, &mut point, InputEvent::TouchUp, 220.);
            assert_eq!(s.gesture.as_ref().unwrap().velocity, velocity);
            velocities.push(velocity);
        }
        assert!((velocities[0] - velocities[1]).abs() < 1.);
    }
    #[test]
    fn kiosk_scroll_append_keeps_the_displayed_position() {
        let position = drag(1600., 200., 1600., 400.);
        assert!(position > 1600.);
        assert_eq!(drag(position, 0., 3200., 400.), position);
    }
    #[test]
    fn kiosk_scroll_spring_settles_under_irregular_frames() {
        for start in [-100., 1700.] {
            let (mut position, mut velocity) = (start, 2000.);
            for n in 0..240 {
                coast(
                    &mut position,
                    &mut velocity,
                    1600.,
                    if n % 2 == 0 { 0.05 } else { 0.008 },
                );
            }
            assert!(position.is_finite());
            assert!(velocity.abs() < 1.);
            assert!((position - position.clamp(0., 1600.)).abs() < 0.5);
        }
    }
}
