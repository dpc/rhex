use super::*;
use crate::game::{Location, fixtures};
use std::collections::HashSet;

#[test]
fn seeded_levels_have_free_spawn_reachable_stairs_and_valid_placements() {
    // A small supervised corpus, not a termination proof or a replay format.
    for level in [0, 1, 3, 8] {
        for seed in [0, 1, 7, 42, 12345] {
            let mut loc = Location::new(level, seed);
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
            let mut duplicate = Location::new(level, seed);
            duplicate.spawn_player(player);
            assert_eq!(
                fixtures::outcome(&duplicate),
                fixtures::outcome(&loc),
                "level {level}, seed {seed}"
            );
        }
    }
}
