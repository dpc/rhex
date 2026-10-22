use super::*;
use crate::game::level::{Biome, LevelId};
use crate::game::{Location, fixtures};
use proptest::prelude::*;
use std::collections::HashSet;

#[test]
fn seeded_levels_have_free_spawn_reachable_stairs_and_valid_placements() {
    // A small supervised corpus, not a termination proof or a replay format.
    for level in [0, 1, 3, 8] {
        for seed in [0, 1, 7, 42, 12345] {
            check_level(level, seed);
        }
    }
}

#[test]
fn seeded_torches_light_free_wall_side_room_tiles() {
    let mut torch_count = 0;
    let mut unlit_room_count = 0;
    for seed in [0, 1, 7, 42, 12345] {
        let mut loc = Location::generated(LevelId(0), 3, Biome::Dungeon, seed);
        let repeat = Location::generated(LevelId(0), 3, Biome::Dungeon, seed);
        loc.recalculate_light_map();
        let mut rooms = HashSet::new();
        let mut lit_rooms = HashSet::new();
        for (&c, t) in loc.map.iter() {
            assert_eq!(*t, repeat.map[c], "seed {seed}, coord {c:?}");
            if let Some(area) = t.area {
                rooms.insert(area.center);
            }
            if t.feature != Some(tile::Torch) {
                assert_eq!(t.light, 0, "no anonymous dungeon light sources");
                continue;
            }
            torch_count += 1;
            let room = t.area.expect("torches belong to rooms");
            lit_rooms.insert(room.center);
            let area::Room(radius) = room.type_;
            assert_eq!(t.light, (radius + 4) as i32);
            assert!(loc.light_map[c] >= t.light as u32);
            assert!(
                c.neighbors()
                    .iter()
                    .any(|&n| loc.map[n].is_passable() && loc.light_map[n] > 0)
            );
            assert_eq!(t.type_, tile::Empty);
            assert!(t.is_passable());
            assert_eq!(t.opaqueness(), tile::Empty.opaqueness());
            assert!(
                c.neighbors()
                    .iter()
                    .any(|&n| loc.map[n].type_ == tile::Wall)
            );
            assert_ne!(c, Coordinate::new(0, 0));
            assert!(!loc.actors_coord_to_id.contains_key(&c));
            assert!(!loc.items.contains_key(&c));
            assert!(!loc.chests.contains_key(&c));
        }
        unlit_room_count += rooms.difference(&lit_rooms).count();
    }
    assert!(torch_count > 0);
    assert!(unlit_room_count > 0, "not every room gets torches");
}

fn check_level(level: u32, seed: u64) {
    let mut loc = Location::generated(LevelId(0), level, Biome::Dungeon, seed);
    let origin = Coordinate::new(0, 0);
    assert!(loc.at(origin).is_passable(), "level {level}, seed {seed}");
    for actor in loc.actors_byid.values() {
        assert!(loc.map[actor.coord()].is_passable());
    }
    for coord in loc.items.keys() {
        // Room centers may hold statues. Pick operates on the head
        // tile, so loot need not be walkable, but must be approachable.
        assert!(
            loc.map[*coord].type_.is_passable()
                && coord.neighbors().iter().any(|c| loc.map[*c].is_passable()),
            "item placement level {level} seed {seed} coord {coord:?} tile {:?}",
            loc.map[*coord]
        );
    }
    let mut visited = HashSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    while let Some(coord) = queue.pop_front() {
        for neighbor in coord.neighbors() {
            let tile = &loc.map[neighbor];
            if (tile.is_passable() || tile.feature == Some(tile::Door(false)))
                && visited.insert(neighbor)
            {
                queue.push_back(neighbor);
            }
        }
    }
    assert!(
        visited
            .iter()
            .any(|c| loc.map[*c].feature == Some(tile::Stairs)),
        "unreachable stairs: level {level}, seed {seed}"
    );
    let mut player = Actor::new(Race::Human, Position::new(origin, Direction::XY));
    player.set_player();
    loc.spawn_player(player.clone());
    fixtures::occupancy(&loc);
    let mut duplicate = Location::generated(LevelId(0), level, Biome::Dungeon, seed);
    duplicate.spawn_player(player);
    assert_eq!(
        fixtures::outcome(&duplicate),
        fixtures::outcome(&loc),
        "level {level}, seed {seed}"
    );
}

proptest! {
    #![proptest_config(crate::property_support::config(
        "generator", include_str!("regressions/generator.txt"), 16))]
    #[test]
    fn bounded_generated_levels(level in 0u32..=8, seed in any::<u64>()) {
        check_level(level, seed);
    }
}
