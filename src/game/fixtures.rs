//! Small worlds and semantic assertions, without procedural setup.

use super::{Actor, Location, Map, actor::Race, tile};
use hex2d::{Coordinate, Position, XY};
use rand::RngExt;

/// Construct a lit room with a human player and real perception.
pub(crate) fn room() -> Location {
    let origin = Coordinate::new(0, 0);
    let mut map = Map::default();
    for coord in origin.range_iter(5) {
        map[coord] = tile::Tile::new(tile::Empty);
        map[coord].light = 8;
    }
    let mut player = Actor::new(Race::Human, Position::new(origin, XY));
    player.set_player();
    let mut loc = Location::fixture(map, player);
    loc.recalculate_light_map();
    let mut player = loc.actors_byid.remove(&0).unwrap();
    player.pre_spawn(&loc);
    player.post_spawn(&loc);
    loc.actors_byid.insert(0, player);
    loc
}

/// Check the bidirectional live occupancy invariant.
pub(crate) fn occupancy(loc: &Location) {
    assert_eq!(loc.actors_coord_to_id.len(), loc.actors_alive_ids().len());
    for id in loc.actors_ids() {
        let actor = &loc.actors_byid[&id];
        if actor.is_dead() {
            assert_ne!(loc.actors_coord_to_id.get(&actor.coord()), Some(&id));
        } else {
            assert_eq!(loc.actors_coord_to_id.get(&actor.coord()), Some(&id));
        }
    }
}

/// Normalize unordered sets/maps only; preserve attack and scheduling order.
pub(crate) fn outcome(loc: &Location) -> String {
    fn entries<T: std::fmt::Debug>(iter: impl Iterator<Item = T>) -> Vec<String> {
        let mut entries: Vec<_> = iter.map(|v| format!("{v:?}")).collect();
        entries.sort();
        entries
    }
    let mut result = format!(
        "{:?}",
        (
            loc.level,
            loc.actors_counter,
            loc.player_id(),
            entries(loc.map.iter()),
            entries(loc.items.iter()),
            entries(loc.light_map.iter()),
            entries(loc.actors_coord_to_id.iter()),
            entries(loc.actors_dead.iter()),
            loc.rng.clone().random::<u64>(),
        )
    );
    for id in loc.actors_ids() {
        let mut actor = loc.actors_byid[&id].clone();
        // Remove only hash-backed fields from Debug; retain every scalar and
        // ordered vector, including private perception/action bookkeeping.
        let unordered = format!(
            "{:?}",
            (
                entries(actor.items_backpack.drain()),
                entries(actor.items_equipped.drain()),
                entries(actor.items_letters.drain()),
                entries(actor.known.drain()),
                entries(actor.visible.drain()),
                entries(actor.in_los.drain()),
                entries(actor.known_areas.drain()),
                entries(actor.discovered.drain()),
                entries(actor.discovered_areas.drain()),
                entries(actor.heard.drain()),
            )
        );
        // temporary_los is private and hash-backed; actor supplies its
        // normalized debug representation for the remaining state.
        result.push_str(&format!("{id}:{unordered}{}", actor.normalized_debug()));
    }
    result
}
