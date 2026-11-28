//! Drag, scroll and view timelines in the engine (LLP 1057.003 D2, D4, D5).
//!
//! A timeline's position is a drag source node's presented `translate` on
//! one axis, or a scroller's offset on one axis as its host presents it
//! ([`Engine::set_scroll_offset`]). A consumer's `animation`s are bound to
//! what its timeline resolved to, over a range of positions. The host
//! resolves names as CSS scopes them (the kernel's `timeline-scope` lookup)
//! and places a scroll or view timeline's range from layout, so the engine
//! hears a node and two positions, never a name. A bound consumer's plays
//! are held (as `animation-play-state: paused` holds them) at the local
//! time the position gives, re-sought whenever a frame is taken or the
//! scroller moves, so a held value, a release spring or a scroll moves the
//! consumer in the frame it moves the source. A sampling host paints it; a
//! lowering host samples the node (its compositor's time is the clock's) or
//! seeks its own executor's copy, as the web's glue does.
//!
//! What a name that finds no single source shows is Chrome's (Scroll-driven
//! Animations 1 §4.2, Web Animations 2's timeline switch):
//! - [`NamedTimeline::Inactive`]: a `timeline-scope` with no declaring
//!   descendant, or several. The animations' current time is unresolved, so
//!   they have no effect: the property's own value shows, whatever the fill.
//! - [`NamedTimeline::Missing`]: no timeline of that name in scope. The
//!   animation has no timeline and keeps the time it has: where its source
//!   left it, still out of effect after an inactive one, 0 if it is new.

use super::Engine;
use crate::property::Property;
use std::collections::{BTreeMap, BTreeSet};

/// What a consumer's timeline name resolved to (LLP 1057.003 D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedTimeline {
    /// The drag timeline this node's presented translate drives.
    Source(u64),
    /// A scroll or view timeline: this scroller's offset on `x` or `y`.
    Scroll {
        /// The scroller's node.
        scroller: u64,
        /// Whether it reads the horizontal offset.
        x: bool,
    },
    /// An inactive timeline: its animations have no effect.
    Inactive,
    /// No timeline: its animations keep the time they have.
    Missing,
}

/// The timelines tracked by the engine. Consumers are indexed by node so
/// detached `Missing` entries never add linear scans to frame ticks or scroll
/// updates.
#[derive(Debug, Default)]
pub(super) struct Timelines {
    /// Each drag source node, and whether it reads the `x` axis.
    sources: Vec<(u64, bool)>,
    /// Each consumer node, what its name resolved to, and its range in points.
    bound: BTreeMap<u64, (NamedTimeline, [f64; 2])>,
    /// Consumer nodes currently bound to `NamedTimeline::Source(_)`.
    drag_bound: BTreeSet<u64>,
    /// Scroller node -> consumer nodes currently bound to it.
    scroll_bound: BTreeMap<u64, BTreeSet<u64>>,
    /// Consumers whose source or scroller was removed; sought lazily on the
    /// next frame unless rebound first (so a commit that removes a scroller and
    /// detaches its consumer preserves the consumer's held progress).
    pending_seek: BTreeSet<u64>,
    /// Each scroller a host has given an offset, and that offset.
    scrolls: Vec<(u64, [f64; 2])>,
}

impl Timelines {
    /// Whether `node` is a timeline's source or follows one.
    pub(super) fn names(&self, node: u64) -> bool {
        self.bound.contains_key(&node)
            || self.sources.iter().any(|s| s.0 == node)
            || self.scrolls.iter().any(|s| s.0 == node)
    }

    fn unindex(&mut self, node: u64, old: NamedTimeline) {
        match old {
            NamedTimeline::Source(_) => {
                self.drag_bound.remove(&node);
            }
            NamedTimeline::Scroll { scroller, .. } => {
                if let Some(set) = self.scroll_bound.get_mut(&scroller) {
                    set.remove(&node);
                    if set.is_empty() {
                        self.scroll_bound.remove(&scroller);
                    }
                }
            }
            NamedTimeline::Inactive | NamedTimeline::Missing => {}
        }
    }

    fn index(&mut self, node: u64, timeline: NamedTimeline) {
        match timeline {
            NamedTimeline::Source(_) => {
                self.drag_bound.insert(node);
            }
            NamedTimeline::Scroll { scroller, .. } => {
                self.scroll_bound.entry(scroller).or_default().insert(node);
            }
            NamedTimeline::Inactive | NamedTimeline::Missing => {}
        }
    }
}

impl Engine {
    /// Declare (or with `None`, retract) the timeline `node`'s presented
    /// translate drives, and whether it reads the `x` axis.
    pub fn set_drag_timeline(&mut self, node: u64, x: Option<bool>) {
        (self.links.set_drag_timeline)(self, node, x)
    }

    pub(super) fn set_drag_timeline_full(&mut self, node: u64, x: Option<bool>) {
        let at = self.timelines.sources.iter().position(|s| s.0 == node);
        match (x, at) {
            (Some(x), Some(i)) => self.timelines.sources[i].1 = x,
            (Some(x), None) => self.timelines.sources.push((node, x)),
            (None, Some(i)) => {
                self.timelines.sources.remove(i);
            }
            (None, None) => return,
        }
        // Drag-bound consumers are sought in `seek_timelines` on the next
        // frame, after any simultaneous `set_animation_timeline` updates in
        // the same commit have taken effect.
    }

    /// `scroller` now stands at `(x, y)`, its scroll offset in points as
    /// its host presents it (overscroll included): the scroll and view
    /// timelines it drives seek their consumers at once. A scroller never
    /// given one stands at its start.
    pub fn set_scroll_offset(&mut self, scroller: u64, x: f64, y: f64) {
        (self.links.set_scroll_offset)(self, scroller, x, y)
    }

    pub(super) fn set_scroll_offset_full(&mut self, scroller: u64, x: f64, y: f64) {
        if !(x.is_finite() && y.is_finite()) {
            return;
        }
        match self.timelines.scrolls.iter_mut().find(|s| s.0 == scroller) {
            Some(s) if s.1 == [x, y] => return,
            Some(s) => s.1 = [x, y],
            None => self.timelines.scrolls.push((scroller, [x, y])),
        }
        let Some(consumers) = self.timelines.scroll_bound.get(&scroller) else {
            return;
        };
        let consumers: Vec<u64> = consumers.iter().copied().collect();
        for consumer in consumers {
            self.seek_timeline(consumer);
        }
    }

    /// Bind (or with `None`, unbind) `node`'s animations to the timeline its
    /// name resolved to, over `range`: at `range[0]` they are at their
    /// start, at `range[1]` at their end.
    pub fn set_animation_timeline(
        &mut self,
        node: u64,
        binding: Option<(NamedTimeline, [f64; 2])>,
    ) {
        (self.links.set_animation_timeline)(self, node, binding)
    }

    pub(super) fn set_animation_timeline_full(
        &mut self,
        node: u64,
        binding: Option<(NamedTimeline, [f64; 2])>,
    ) {
        if !self.timelines.pending_seek.is_empty() {
            self.timelines.pending_seek.remove(&node);
        }
        let previous = self.timelines.bound.get(&node).copied();
        match (binding, previous) {
            (Some((timeline, range)), Some((old_timeline, _))) => {
                if old_timeline != timeline {
                    self.timelines.unindex(node, old_timeline);
                    self.timelines.index(node, timeline);
                }
                self.timelines.bound.insert(node, (timeline, range));
                self.seek_timeline(node);
            }
            (Some((timeline, range)), None) => {
                self.timelines.index(node, timeline);
                self.timelines.bound.insert(node, (timeline, range));
                self.seek_timeline(node);
            }
            (None, Some((old_timeline, _))) => {
                self.timelines.unindex(node, old_timeline);
                self.timelines.bound.remove(&node);
                // Back on the clock: a play detached from the clock keeps its
                // original `start` (Web Animations 2 §4.4.4); one from a
                // progress timeline resumes where it was left; one from an
                // inactive timeline starts at 0. With the first-frame rule on,
                // that resume waits for the frame (LLP 1003.001 D3); a clock's
                // join below decides for its own.
                let now = self.sample_time();
                let pending = self.holds().then_some(now);
                let mut dirtied = false;
                for play in self
                    .animations
                    .get_mut(&node)
                    .into_iter()
                    .flatten()
                    .filter(|p| !p.exit)
                {
                    if !play.animation.paused {
                        let detached = std::mem::take(&mut play.detached_clock);
                        if let Some(held) = play.hold.take() {
                            if !detached {
                                play.start = now - if held.is_nan() { 0.0 } else { held };
                                play.pending = pending;
                            }
                            play.clocked = play.pending.is_none();
                            dirtied = true;
                        }
                    }
                }
                if dirtied {
                    let properties: Vec<Property> = self.animations[&node]
                        .iter()
                        .flat_map(|play| play.animation.keyframes.properties())
                        .filter(|p| !self.lowered_for(node, *p))
                        .collect();
                    for property in properties {
                        self.dirty.insert((node, property));
                    }
                }
                // Binding took the node off the clock's list; a live play
                // goes back on it, waiting for the frame or not.
                self.schedule_animations(node);
                // Onto a clock timeline: in its phase (LLP 1055.002).
                self.join_clock_node(node);
            }
            (None, None) => {}
        }
    }

    /// Whether `node`'s animations follow a timeline instead of the clock.
    pub fn timeline_bound(&self, node: u64) -> bool {
        self.timelines.bound.contains_key(&node)
    }

    /// What timeline `node` is currently bound to, if any.
    pub(super) fn timeline_kind(&self, node: u64) -> Option<NamedTimeline> {
        self.timelines.bound.get(&node).map(|(t, _)| *t)
    }

    pub(super) fn forget_timelines(&mut self, node: u64) {
        let had_scroll = self.timelines.scrolls.iter().any(|s| s.0 == node);
        self.timelines.sources.retain(|s| s.0 != node);
        if let Some((old, _)) = self.timelines.bound.remove(&node) {
            self.timelines.unindex(node, old);
        }
        if !self.timelines.pending_seek.is_empty() {
            self.timelines.pending_seek.remove(&node);
        }
        self.timelines.scrolls.retain(|s| s.0 != node);
        // Defer re-seeking scroll consumers whose scroller was forgotten until
        // the next frame: if the same commit also switches the consumer to
        // `Missing` (`none`) or `auto`, its binding update clears `pending_seek`
        // and preserves the consumer's held progress.
        if had_scroll {
            if let Some(consumers) = self.timelines.scroll_bound.get(&node) {
                self.timelines
                    .pending_seek
                    .extend(consumers.iter().copied());
            }
        }
    }

    /// Hold every drag-bound consumer's plays at the time its timeline gives.
    pub(super) fn seek_timelines(&mut self) {
        (self.links.seek_timelines)(self)
    }

    pub(super) fn seek_timelines_full(&mut self) {
        if !self.timelines.pending_seek.is_empty() {
            let pending = std::mem::take(&mut self.timelines.pending_seek);
            for consumer in pending {
                if self.timelines.bound.contains_key(&consumer) {
                    self.seek_timeline(consumer);
                }
            }
        }
        let drag_nodes: Vec<u64> = self.timelines.drag_bound.iter().copied().collect();
        for node in drag_nodes {
            self.seek_timeline(node);
        }
    }

    /// Mark newly started unbound plays that are actively running on the
    /// document timeline (not held and not waiting on a first frame) so a later
    /// switch to `none` or a missing timeline makes their current time
    /// unresolved while keeping their clock start (Web Animations 2 §4.4.4).
    pub(super) fn mark_clocked_animations(&mut self) {
        if self.newly_clocked.is_empty() {
            return;
        }
        for node in std::mem::take(&mut self.newly_clocked) {
            if let Some(plays) = self.animations.get_mut(&node) {
                for play in plays.iter_mut() {
                    if !play.exit && play.hold.is_none() && play.pending.is_none() {
                        play.clocked = true;
                    }
                }
            }
        }
    }

    /// Re-seek a bound node whose plays were just set.
    pub(super) fn seek_bound(&mut self, node: u64) {
        if self.timelines.bound.contains_key(&node) {
            self.seek_timeline(node);
        }
    }

    fn seek_timeline(&mut self, node: u64) {
        let Some(&(timeline, [a, b])) = self.timelines.bound.get(&node) else {
            return;
        };
        // The source's presented translate on its axis, as progress through
        // the range; `None` for no timeline, NaN for an inactive one. A
        // source the engine has not heard of yet is no timeline until it is.
        let progress = match timeline {
            NamedTimeline::Source(s) => self
                .timelines
                .sources
                .iter()
                .find(|x| x.0 == s)
                .and_then(|&(_, x)| Some((self.value(s, Property::Translate)?, x)))
                .map(|(v, x)| {
                    // Unclamped: outside the range the animation is before or
                    // after its interval, and its fill decides, as on a CSS
                    // scroll timeline.
                    let p = ((if x { v.x } else { v.y }) - a) / (b - a);
                    if p.is_finite() {
                        p
                    } else {
                        0.0
                    }
                }),
            // Unclamped, as a drag's: the rubber band past either end moves
            // the consumer on, and its fill decides outside the range. A
            // zero-length range has no progress, an inactive timeline's.
            NamedTimeline::Scroll { scroller, x } => {
                let at = (self.timelines.scrolls.iter())
                    .find(|s| s.0 == scroller)
                    .map_or(0.0, |s| s.1[usize::from(!x)]);
                Some(if a == b { f64::NAN } else { (at - a) / (b - a) })
            }
            NamedTimeline::Inactive => Some(f64::NAN),
            NamedTimeline::Missing => None,
        };
        let now = self.sample_time();
        let Some(plays) = self.animations.get_mut(&node) else {
            return;
        };
        let mut moved = false;
        for play in plays.iter_mut().filter(|p| !p.exit) {
            // A paused animation keeps a resolved finite hold time across
            // timeline changes and progress updates (Web Animations 2 pause
            // model). An unresolved hold (`None` or `Some(NaN)`) is still
            // waiting for its first active timeline sample.
            if play.animation.paused && play.hold.is_some_and(|h| !h.is_nan()) {
                continue;
            }
            // The range spans the delay and the active interval together, as
            // Web Animations 2 converts an animation's times on a
            // progress-based timeline (Chrome agrees: the parity fixture's
            // `tl-delay`). An endless animation has no end to span and holds
            // its start, a declared deviation: CSS gives it no duration and
            // shows its end. An unresolved time stays unresolved. Detaching a
            // running clock play onto no timeline (`Missing`) makes its time
            // unresolved while preserving `start` (Web Animations 2 §4.4.4).
            let end = play.animation.end_time();
            let local = match progress {
                Some(p) if p.is_nan() => {
                    play.detached_clock = false;
                    p
                }
                Some(p) if end.is_finite() => {
                    play.detached_clock = false;
                    p * end
                }
                Some(_) => {
                    play.detached_clock = false;
                    play.animation.delay.max(0.0)
                }
                None => match play.hold {
                    Some(held) => held,
                    None if play.pending.is_some() => play.local(now),
                    None if play.clocked || now > play.start => {
                        play.detached_clock = true;
                        f64::NAN
                    }
                    None => 0.0,
                },
            };
            play.clocked = false;
            // The source owns its time: nothing waits for a frame.
            play.pending = None;
            moved |= play.hold.replace(local).map(f64::to_bits) != Some(local.to_bits());
        }
        self.schedule_animations(node);
        if !moved {
            return;
        }
        // A lowering host's executor seeks its own copy (the web's glue); only
        // what the engine samples is painted from here.
        let properties: Vec<Property> = self.animations[&node]
            .iter()
            .filter(|play| !play.exit)
            .flat_map(|play| play.animation.keyframes.properties())
            .filter(|p| !self.lowered_for(node, *p))
            .collect();
        for property in properties {
            self.dirty.insert((node, property));
        }
    }
}
