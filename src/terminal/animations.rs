use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::game::combat_event::CombatEvent;
use hex2d::Coordinate;

/// Presentation-only effects, ordered by resolution within each impacted cell.
#[derive(Default)]
pub(super) struct Animations {
    /// Pending and playing effects; elapsed entries are removed on advance.
    active: Vec<Hit>,
    /// Time-bucket unions retain every source without per-source playback queues.
    sounds: BTreeMap<u128, SoundMask>,
    /// Actor-bound acquisition markers, concurrent with hits and sound waves.
    awareness: BTreeMap<crate::game::actor::Id, Duration>,
}

/// One 100ms presentation slice, coalescing overlapping waves and origins.
#[derive(Default)]
struct SoundMask {
    /// Visible-at-capture wave coordinates.
    waves: BTreeSet<(i32, i32)>,
    /// Unseen-at-capture audible origins.
    markers: BTreeSet<(i32, i32)>,
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
/// Six 100ms slices keep wave cells bold three times as long as before.
const SOUND_BOLD_TICKS: u128 = 6;

impl Animations {
    /// Start visible acquisition markers together, without adding gameplay delays.
    pub(super) fn enqueue_awareness(
        &mut self,
        events: Vec<crate::game::awareness_event::AwarenessEvent>,
        now: Duration,
    ) {
        self.advance(now);
        for event in events {
            self.awareness
                .insert(event.actor, now + Duration::from_millis(500));
        }
    }

    /// Yield identities while their brief yellow marker is active.
    pub(super) fn aware_actors(
        &self,
        now: Duration,
    ) -> impl Iterator<Item = crate::game::actor::Id> + '_ {
        self.awareness
            .iter()
            .filter_map(move |(&id, &end)| (now < end).then_some(id))
    }

    /// Start every sound in a turn together; bucket quantization shortens by <100ms.
    pub(super) fn enqueue_sounds(
        &mut self,
        sounds: Vec<crate::game::sound::ObservedSound>,
        now: Duration,
    ) {
        self.advance(now);
        let start = now.as_millis() / 100;
        let max_hops = sounds
            .iter()
            .flat_map(|s| &s.cells)
            .map(|c| c.hops)
            .max()
            .unwrap_or(1)
            .max(1);
        for sound in sounds {
            for cell in sound.cells {
                let phase = u128::from(3 * cell.hops / max_hops);
                // Keep arrival phases unchanged; only extend the trailing highlight.
                for tick in start + phase..start + phase + SOUND_BOLD_TICKS {
                    self.sounds
                        .entry(tick)
                        .or_default()
                        .waves
                        .insert((cell.coord.x, cell.coord.y));
                }
            }
            if let Some(source) = sound.hidden_source {
                for tick in start..start + 5 {
                    self.sounds
                        .entry(tick)
                        .or_default()
                        .markers
                        .insert((source.x, source.y));
                }
            }
        }
    }

    /// Query precomputed masks without touching simulation or propagating sound.
    pub(super) fn sound_cells(
        &self,
        now: Duration,
    ) -> impl Iterator<Item = (Coordinate, bool)> + '_ {
        self.sounds
            .get(&(now.as_millis() / 100))
            .into_iter()
            .flat_map(|mask| {
                mask.waves
                    .iter()
                    .map(|&(x, y)| (Coordinate::new(x, y), false))
                    .chain(
                        mask.markers
                            .iter()
                            .map(|&(x, y)| (Coordinate::new(x, y), true)),
                    )
            })
    }

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
        self.sounds.retain(|&tick, _| tick >= now.as_millis() / 100);
        self.awareness.retain(|_, end| now < *end);
    }

    /// Discard effects when their location or session is replaced.
    pub(super) fn clear(&mut self) {
        self.active.clear();
        self.sounds.clear();
        self.awareness.clear();
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
