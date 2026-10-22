//! Bounded reward placement on the entrance's walkable component.
use std::collections::{HashSet, VecDeque};

use hex2d::Coordinate;
use rand::{RngExt, seq::SliceRandom};

use super::{Location, chest::Chest, item, level::Biome, tile};

/// Populate already-generated terrain without overwriting exits or blocking routes.
pub(super) fn populate(loc: &mut Location) {
    let origin = Coordinate::new(0, 0);
    let mut reachable = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    while let Some(c) = queue.pop_front() {
        for n in c.neighbors() {
            if loc.map[n].is_passable() && reachable.insert(n) {
                queue.push_back(n);
            }
        }
    }
    let mut candidates: Vec<_> = reachable
        .into_iter()
        .filter(|&c| {
            c.distance(origin) > 3
                && matches!(loc.map[c].type_, tile::Empty | tile::Ground)
                && loc.map[c].feature.is_none()
                && !loc.actors_coord_to_id.contains_key(&c)
        })
        .collect();
    candidates.sort_by_key(|c| (c.x, c.y));
    candidates.shuffle(&mut loc.rng);
    match loc.biome {
        Biome::DarkForest => {
            let mut placed: Vec<Coordinate> = Vec::new();
            for c in candidates {
                if loc.items.contains_key(&c) || placed.iter().any(|p| p.distance(c) < 4) {
                    continue;
                }
                if placed.is_empty() {
                    loc.chests.insert(c, roll_chest(loc.level, &mut loc.rng));
                } else {
                    loc.items
                        .insert(c, item::exploration_loot(loc.level.min(2), &mut loc.rng));
                }
                placed.push(c);
                if placed.len() == 3 {
                    break;
                }
            }
        }
        Biome::Dungeon => {
            // Replace reward sites rather than independently doubling their count.
            for &c in &candidates {
                if loc.chests.len() == 6 {
                    break;
                }
                if loc.items.contains_key(&c) && loc.rng.random_bool(1.0 / 3.0) {
                    loc.items.remove(&c);
                    loc.chests.insert(c, roll_chest(loc.level, &mut loc.rng));
                }
            }
            if loc.chests.is_empty()
                && let Some(&c) = candidates.first()
            {
                loc.items.remove(&c);
                loc.chests.insert(c, roll_chest(loc.level, &mut loc.rng));
            }
        }
    }
}

fn roll_chest(level: i32, rng: &mut impl rand::Rng) -> Chest {
    let count = match rng.random_range(0..100) {
        0..50 => 0,
        50..85 => 1,
        _ => 2,
    };
    Chest::new((0..count).map(|_| item::exploration_loot(level, rng)))
}
