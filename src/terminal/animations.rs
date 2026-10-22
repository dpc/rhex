use std::time::Duration;

use crate::game::combat_event::CombatEvent;
use hex2d::Coordinate;

/// Presentation-only effects, ordered by resolution within each impacted cell.
#[derive(Default)]
pub(super) struct Animations {
    /// Pending and playing effects; elapsed entries are removed on advance.
    active: Vec<Hit>,
}

/// One strike followed by confirmation, anchored to immutable impact geometry.
struct Hit {
    /// Resolved geometry, never an actor lookup.
    event: CombatEvent,
    /// Start time, after earlier effects at the same cell.
    start: Duration,
}

const STRIKE: Duration = Duration::from_millis(200);
const TOTAL: Duration = Duration::from_millis(400);

impl Animations {
    /// Queue observed hits without coalescing or delaying the simulation.
    pub(super) fn enqueue(&mut self, events: Vec<CombatEvent>, now: Duration) {
        self.advance(now);
        for event in events {
            let start = self
                .active
                .iter()
                .filter(|hit| hit.event.impact == event.impact)
                .map(|hit| hit.start + TOTAL)
                .max()
                .unwrap_or(now)
                .max(now);
            self.active.push(Hit { event, start });
        }
    }

    /// Free completed effects even when a modal screen hides the map.
    pub(super) fn advance(&mut self, now: Duration) {
        self.active.retain(|hit| now < hit.start + TOTAL);
    }

    /// Discard effects when their location or session is replaced.
    pub(super) fn clear(&mut self) {
        self.active.clear();
    }

    /// Yield the current glyph of each playing effect in resolution order.
    pub(super) fn glyphs(&self, now: Duration) -> impl Iterator<Item = (Coordinate, char)> + '_ {
        self.active.iter().filter_map(move |hit| {
            let age = now.checked_sub(hit.start)?;
            if age >= TOTAL {
                return None;
            }
            let glyph = if age >= STRIKE {
                '*'
            } else {
                // Screen-space slope works for all hex directions and ranged shots.
                let (x, y) =
                    (hit.event.impact - hit.event.source).to_pixel_integer(super::consts::SPACING);
                if y == 0 {
                    '-'
                } else if (x >= 0) == (y >= 0) {
                    '\\'
                } else {
                    '/'
                }
            };
            Some((hit.event.impact, glyph))
        })
    }
}

#[cfg(test)]
#[path = "animations_tests.rs"]
mod tests;
