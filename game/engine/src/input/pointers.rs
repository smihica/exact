//! Every pointer contact, not only the primary one (`Input::pointer`): each
//! touch, and the mouse while a button is held, with the edges where each
//! began and ended this tick. A contact still down is in the saved `Input`'s
//! contacts already, so a save holds it without a field of its own.
use super::{Input, PointerPhase};
use crate::{Data, Vec2};

/// One contact as the tick reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct PointerContact {
    /// Platform contact identifier, unique among the contacts down.
    pub id: u64,
    /// Latest canvas position in points.
    pub position: Vec2,
    /// Where the contact went down; a mouse's latest MouseLeft press while it
    /// is held.
    pub press_origin: Vec2,
    /// False only for a contact that lifted or was cancelled this tick: it is
    /// gone at the next.
    pub down: bool,
    /// Device motion accumulated since the last tick, in points.
    pub delta: Vec2,
}
/// A contact's beginning (`Down`) or end (`Up`, `Cancel`) this tick. A move
/// is not an edge: its position is the contact's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Data)]
pub struct PointerEdge {
    /// Platform contact identifier.
    pub id: u64,
    /// `Down`, `Up` or `Cancel`.
    pub phase: PointerPhase,
    /// Canvas position in points at the edge.
    pub position: Vec2,
    /// World microseconds at which it happened.
    pub world_us: i64,
}
impl Input {
    /// Every contact down, and each that ended this tick, sorted by id.
    pub fn pointers(&self) -> &[PointerContact] {
        &self.pointers
    }
    /// This tick's contact edges, in the order they happened.
    pub fn pointer_events(&self) -> &[PointerEdge] {
        &self.pointer_edges
    }
    /// Follow one pointer event as the raw contacts do: a Down begins a
    /// contact, an Up with no button left or a Cancel ends it.
    pub(super) fn track_pointer(
        &mut self,
        edge: PointerEdge,
        motion: Vec2,
        ends: bool,
        primary_down: bool,
    ) {
        let PointerEdge {
            id,
            phase,
            position,
            ..
        } = edge;
        let at = self.pointers.binary_search_by_key(&id, |p| p.id);
        match (phase, at) {
            (PointerPhase::Down, Ok(i)) if self.pointers[i].down => {
                let p = &mut self.pointers[i];
                p.position = position;
                p.delta += motion;
                if primary_down {
                    p.press_origin = position;
                }
            }
            (PointerPhase::Down, _) => {
                let contact = PointerContact {
                    id,
                    position,
                    press_origin: position,
                    down: true,
                    delta: motion,
                };
                // An id that ended earlier this tick begins again.
                match at {
                    Ok(i) => self.pointers[i] = contact,
                    Err(i) => self.pointers.insert(i, contact),
                }
                self.pointer_edges.push(edge);
            }
            (_, Ok(i)) if self.pointers[i].down => {
                let p = &mut self.pointers[i];
                p.position = position;
                p.delta += motion;
                if ends {
                    p.down = false;
                    self.pointer_edges.push(edge);
                }
            }
            _ => {}
        }
    }
    /// Blur cancels every contact down where it last was.
    pub(super) fn cancel_pointers(&mut self, world_us: i64) {
        for p in self.pointers.iter_mut().filter(|p| p.down) {
            p.down = false;
            self.pointer_edges.push(PointerEdge {
                id: p.id,
                phase: PointerPhase::Cancel,
                position: p.position,
                world_us,
            });
        }
    }
    /// The tick's edges and ended contacts expire; motion starts again.
    pub(super) fn clear_pointer_edges(&mut self) {
        self.pointer_edges.clear();
        self.pointers.retain(|p| p.down);
        for p in &mut self.pointers {
            p.delta = Vec2::ZERO;
        }
    }
    /// The contacts down, from a restored `Input`'s raw contacts.
    pub(super) fn restore_pointers(&mut self) {
        self.pointer_edges.clear();
        self.pointers = self
            .contacts
            .iter()
            .filter(|c| c.action.is_empty())
            .map(|c| PointerContact {
                id: c.id,
                position: c.position,
                press_origin: c.origin,
                down: true,
                delta: Vec2::ZERO,
            })
            .collect();
        self.pointers.sort_by_key(|p| p.id);
    }
}
