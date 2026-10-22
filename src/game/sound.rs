//! Bounded additive acoustics shared by hearing and presentation.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use hex2d::{Coordinate, Direction};

use super::{Items, Map, Noise};

#[cfg(test)]
thread_local! {
    /// Per-thread propagation calls, settled cells and examined edges for tests.
    pub(crate) static WORK: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}

/// Immutable action-time emission, including sounds from dead/removed actors.
#[derive(Clone, Debug)]
pub struct Emission {
    /// Position at emission, not the actor's later position.
    pub source: Coordinate,
    /// Existing hearing/log identity.
    pub kind: Noise,
    /// Inclusive open-floor radius; supported range is 1..=16.
    pub level: i32,
}

/// One reached cell along its least-attenuated path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cell {
    /// Reached position.
    pub coord: Coordinate,
    /// Quarter-level path cost.
    pub spent: u16,
    /// Path length, breaking equal-cost ties toward fewer steps.
    pub hops: u16,
}

/// Player-permitted presentation data, independent of actor lifetime.
#[derive(Clone, Debug)]
pub struct ObservedSound {
    /// Only cells visible when the sound was resolved.
    pub cells: Vec<Cell>,
    /// Audible origin unseen at capture; reveals no underlying world content.
    pub hidden_source: Option<Coordinate>,
}

/// Flood at most the radius-16 hex disk (817 cells), never the entire map.
pub fn propagate(map: &Map, items: &Items, emission: &Emission) -> Vec<Cell> {
    #[cfg(test)]
    WORK.set({
        let (calls, cells, edges) = WORK.get();
        (calls + 1, cells, edges)
    });
    if emission.level <= 0 || !map[emission.source].transmits_sound() {
        return Vec::new();
    }
    debug_assert!(emission.level <= 16);
    let budget = 4 * emission.level.min(16) as u16;
    let source = emission.source;
    let mut best = HashMap::from([(source, (0u16, 0u16))]);
    let mut queue = BinaryHeap::from([Reverse((0u16, 0u16, source.x, source.y))]);
    let mut cells = Vec::new();
    while let Some(Reverse((spent, hops, x, y))) = queue.pop() {
        let coord = Coordinate::new(x, y);
        if best[&coord] != (spent, hops) {
            continue;
        }
        cells.push(Cell { coord, spent, hops });
        #[cfg(test)]
        WORK.set({
            let (calls, cells, edges) = WORK.get();
            (calls, cells + 1, edges)
        });
        if spent + 4 > budget {
            continue;
        }
        for &direction in Direction::all() {
            #[cfg(test)]
            WORK.set({
                let (calls, cells, edges) = WORK.get();
                (calls, cells, edges + 1)
            });
            let next = coord + direction;
            if !map[next].transmits_sound() {
                continue;
            }
            let damping = std::iter::once(next)
                .chain(Direction::all().iter().map(|&d| next + d))
                .map(|c| {
                    map[c]
                        .sound_dampening()
                        .max(items.get(&c).map_or(0, |item| item.sound_dampening()))
                })
                .max()
                .unwrap();
            let cost = (spent + 4 + damping, hops + 1);
            if cost.0 <= budget && best.get(&next).is_none_or(|&old| cost < old) {
                best.insert(next, cost);
                queue.push(Reverse((cost.0, cost.1, next.x, next.y)));
            }
        }
    }
    cells.sort_unstable_by_key(|c| (c.hops, c.coord.x, c.coord.y));
    cells
}

#[cfg(test)]
#[path = "sound_tests.rs"]
mod tests;
