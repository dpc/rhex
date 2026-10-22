//! Bounded, seeded forest growth: dense vegetation, clustered trunks, water,
//! small canopy openings, and narrow routes connecting those openings.
use std::collections::{HashMap, VecDeque};

use hex2d::{Coordinate, Direction, Position};
use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::game::{
    Actors, Items, Map,
    actor::{Actor, Race},
    tile::{self, Tile},
};

#[cfg(test)]
#[path = "forest_tests.rs"]
mod tests;

/// Forest radius bounds all painting and route searches.
pub const RADIUS: i32 = 16;

/// Generate a forest without retries; routes may cross shallows, never deep water.
pub fn generate(depth: u32, seed: u64) -> (Map, Actors, Items) {
    let mut rng = SmallRng::seed_from_u64(seed);
    // Shape draws must not move the creek, grove candidates, or trail RNG stream.
    let mut shapes = rng.clone();
    let origin = Coordinate::new(0, 0);
    let mut map = Map::default();
    for c in origin.range_iter(RADIUS) {
        map[c] = Tile::new(tile::Thicket);
    }

    // Large trunks form groves rather than independent single-cell obstacles.
    let mut groves: Vec<(Coordinate, i32)> = Vec::new();
    for _ in 0..80 {
        let center = Coordinate::new(rng.random_range(-14..=14), rng.random_range(-14..=14));
        let radius = rng.random_range(1..=2);
        if origin.distance(center) + radius > RADIUS
            || groves
                .iter()
                .any(|&(c, r)| c.distance(center) <= r + radius + 1)
        {
            continue;
        }
        groves.push((center, radius));
        paint_formation(&mut map, center, radius, tile::TreeTrunk, &mut shapes);
    }

    // A meandering creek has a shallow bank and intermittent deep channel.
    let mut x = 5;
    let mut channel = Vec::new();
    for y in -RADIUS..=RADIUS {
        x = (x + rng.random_range(-1..=1)).clamp(3, 7);
        let c = Coordinate::new(x, y);
        paint(&mut map, c, 1, tile::ShallowWater);
        if origin.distance(c) <= RADIUS && y % 4 != 0 {
            channel.push(c);
        }
    }
    for c in channel {
        map[c] = Tile::new(tile::Water);
    }
    let pond = Coordinate::new(-8, 7 + rng.random_range(-1..=1));
    paint_formation(&mut map, pond, 3, tile::ShallowWater, &mut shapes);
    paint(&mut map, pond, 1, tile::Water);

    let clearings = [
        origin,
        Coordinate::new(-10, 2),
        Coordinate::new(-3, -8),
        Coordinate::new(9, -10),
        Coordinate::new(12, -3),
    ];
    // Openings have dark edges and a single small source of canopy light.
    for &center in &clearings {
        paint_formation(&mut map, center, 2, tile::Ground, &mut shapes);
        map[center].light = 5;
    }
    // Water and openings can clip a grove; don't leave isolated trunk pixels.
    let fragments: Vec<_> = origin
        .range_iter(RADIUS)
        .filter(|&c| {
            map[c].type_ == tile::TreeTrunk
                && c.neighbors()
                    .iter()
                    .all(|&n| map[n].type_ != tile::TreeTrunk)
        })
        .collect();
    for c in fragments {
        map[c] = Tile::new(tile::Thicket);
    }
    // Connect openings through vegetation. If a trunk or channel divides them,
    // a bounded search follows the bank instead of cutting the trunk in half.
    for pair in clearings.windows(2) {
        connect(&mut map, pair[0], pair[1]);
    }
    // Short branching trails soften the solid undergrowth without opening rooms.
    for &center in &clearings {
        let mut c = center;
        for _ in 0..45 {
            let neighbors = c.neighbors();
            let next = neighbors[rng.random_range(0..6)];
            if origin.distance(next) < RADIUS
                && matches!(map[next].type_, tile::Ground | tile::Thicket)
            {
                c = next;
                map[c].type_ = tile::Ground;
            }
        }
    }
    map[*clearings.last().unwrap()].feature = Some(tile::ForestPath);

    let mut actors = Actors::default();
    for &center in clearings.iter().skip(1).take(3) {
        let c = center.neighbors()[0];
        actors.insert(
            c,
            Actor::new(
                match rng.random_range(0..4) {
                    0 => Race::Wolf,
                    1 => Race::CaveBeetle,
                    _ if depth < 3 => Race::Rat,
                    _ => Race::Goblin,
                },
                Position::new(c, Direction::XY),
            ),
        );
    }
    // SimpleMap keeps the most recent write pending; flush before iteration.
    map.compact();
    (map, actors, Items::default())
}

fn paint(map: &mut Map, center: Coordinate, radius: i32, terrain: tile::Type) {
    for c in center.range_iter(radius) {
        if Coordinate::new(0, 0).distance(c) <= RADIUS {
            map[c] = Tile::new(terrain);
        }
    }
}

/// Overlapping lobes give larger formations uneven edges without pixel noise.
/// Keep a solid radius-one core for spawns/light, and stay inside the original
/// envelope so separated groves and the pond's shallow bank remain intact.
fn paint_formation(
    map: &mut Map,
    center: Coordinate,
    radius: i32,
    terrain: tile::Type,
    rng: &mut SmallRng,
) {
    if radius == 1 {
        paint(map, center, radius, terrain);
        return;
    }
    paint(map, center, radius - 1, terrain);
    let neighbors = center.neighbors();
    let first = rng.random_range(0..6);
    let second = (first + rng.random_range(1..=2)) % 6;
    for index in [first, second] {
        paint(map, neighbors[index], radius - 1, terrain);
    }
}

fn connect(map: &mut Map, start: Coordinate, end: Coordinate) {
    let mut parents = HashMap::from([(start, start)]);
    let mut queue = VecDeque::from([start]);
    while let Some(c) = queue.pop_front() {
        if c == end {
            break;
        }
        for n in c.neighbors() {
            if (map[n].is_passable() || map[n].type_ == tile::Thicket)
                && let std::collections::hash_map::Entry::Vacant(entry) = parents.entry(n)
            {
                entry.insert(c);
                queue.push_back(n);
            }
        }
    }
    // The shallow banks and separated groves leave a connected substrate.
    assert!(parents.contains_key(&end), "forest route disconnected");
    let mut c = end;
    while c != start {
        if map[c].type_ == tile::Thicket {
            map[c].type_ = tile::Ground;
        }
        c = parents[&c];
    }
}
