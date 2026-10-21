use fnv::FnvHasher;
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use std::collections::{HashMap, VecDeque};
use std::hash::BuildHasherDefault;

use crate::game::actor::{Actor, Race};
use crate::game::area;
use crate::game::item;
use crate::game::tile;
use crate::game::{Actors, Items, Map};
use hex2d as h2d;
use hex2d::Angle::*;
use hex2d::{Coordinate, Direction, Position};

type EndpointQueue = VecDeque<h2d::Position>;

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;

pub struct DungeonGenerator {
    level: u32,
    start: Option<Coordinate>,
    stairs: Option<Coordinate>,
    tile_count: u32,
    map: HashMap<Coordinate, tile::Tile, BuildHasherDefault<FnvHasher>>,
    endpoints: EndpointQueue,
    actors: Actors,
    items: Items,
    rng: SmallRng,
}

impl DungeonGenerator {
    /// Create a generator with an explicit random stream.
    pub fn new(level: u32, seed: u64) -> DungeonGenerator {
        DungeonGenerator {
            level,
            start: None,
            stairs: None,
            tile_count: 0,
            map: Default::default(),
            endpoints: VecDeque::new(),
            actors: Default::default(),
            items: Default::default(),
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

fn tile_is_deadend(map: &Map, coord: Coordinate) -> bool {
    let neighbors = coord.neighbors();

    let passable: Vec<bool> = neighbors
        .iter()
        .map(|n_coord| map[*n_coord].is_passable())
        .collect();

    let len = passable.len();

    assert_eq!(len, 6);

    let mut changes = 0;
    let mut last = passable[len - 1];
    for current in passable {
        if last != current {
            changes += 1;
            last = current;
        }
    }

    changes < 4
}

impl DungeonGenerator {
    // generate_map_feature
    fn generate_continue_corridor(&mut self, pos: h2d::Position) {
        let npos = pos + Coordinate::from(pos.dir);

        let somepos = self.map.get(&npos.coord).map(|tile| {
            if tile.type_.is_passable() {
                npos
            } else {
                pos + Right
            }
        });
        match somepos {
            Some(pos) => self.endpoint_push(pos),
            None => {
                self.map.insert(npos.coord, tile::Tile::new(tile::Empty));
                self.endpoint_push(npos);
                if self.rng.random_range(0..19) == 0 {
                    let leftwall = pos + Coordinate::from(pos.dir + h2d::Angle::Left);
                    let rightwall = pos + Coordinate::from(pos.dir + h2d::Angle::Right);

                    self.map
                        .entry(leftwall.coord)
                        .or_insert(tile::Tile::new(tile::Wall));
                    self.map
                        .entry(rightwall.coord)
                        .or_insert(tile::Tile::new(tile::Wall));
                }

                self.tile_count += 1
            }
        }
    }

    // generate_map_feature
    fn generate_turn(&mut self, pos: h2d::Position, turn: h2d::Angle) {
        self.generate_continue_corridor(pos + turn)
    }

    // generate_map_feature
    fn generate_cross(&mut self, pos: h2d::Position, turn: h2d::Angle) {
        self.endpoint_push(pos + turn);
        self.generate_continue_corridor(pos)
    }

    /// Generate room in front of the iterator `(pos, dir)`
    fn generate_room(&mut self, pos: h2d::Position, r: u32) {
        self.endpoint_push(pos);

        let center_pos = pos + Coordinate::from(pos.dir).scale(r as i32);

        let tile_count_old = self.tile_count;
        self.generate_room_inplace(center_pos, r);

        if tile_count_old == self.tile_count {
            match self.rng.random_range(0..8) {
                0 => self.endpoint_push(pos + Left),
                1 => self.endpoint_push(pos + LeftBack),
                2 => self.endpoint_push(pos + Right),
                3 => self.endpoint_push(pos + RightBack),
                _ => {}
            }
        }
    }

    // generate_map at position `pos`; does not push back the iterator!
    fn generate_room_inplace(&mut self, center: h2d::Position, r: u32) {
        let coord = center.coord;

        let mut blocked = false;
        for c in coord.range_iter((r - 1) as i32) {
            if self.map.contains_key(&c) {
                blocked = true;
            }
        }

        if blocked {
            return;
        }

        let area = area::Area::new(coord, area::Type::Room(r));

        if Some(coord) != self.start {
            match self.rng.random_range(0..6) {
                2 => {
                    if self.stairs.is_none() {
                        self.map.insert(
                            coord,
                            *tile::Tile::new(tile::Empty)
                                .add_feature(tile::Stairs)
                                .add_area(area),
                        );
                        self.stairs = Some(coord);
                        self.tile_count += 1;
                    }
                }
                3 => {
                    self.map.insert(
                        coord,
                        *tile::Tile::new(tile::Empty)
                            .add_feature(tile::Statue)
                            .add_area(area),
                    );
                    self.tile_count += 1;
                }
                _ => {}
            }
        }

        for c in coord.range_iter((r - 1) as i32) {
            if !self.map.contains_key(&c) {
                self.tile_count += 1;
                self.map
                    .insert(c, *tile::Tile::new(tile::Empty).add_area(area));
            }
        }

        // TODO: Guarantee that the room is not completely closed
        for c in coord.ring_iter(r as i32, h2d::Spin::CW(h2d::Direction::XY)) {
            if !self.map.contains_key(&c) {
                self.tile_count += 1;
                self.map.insert(
                    c,
                    *tile::Tile::new(tile::Empty).add_feature(tile::Door(false)),
                );
            }
        }

        for c in coord.range_iter(r as i32 - 1) {
            if self.map.contains_key(&c) {
                if self.rng.random_range(0..15) == 0 {
                    self.map.get_mut(&c).unwrap().add_light((r + 4) as i32);
                }
                self.tile_count += 1;
            }
        }

        for c in coord.range_iter(r as i32 / 2) {
            if c != coord
                && self.map.get(&c).is_some_and(|t| t.is_passable())
                && self.rng.random_range(0..10) == 0
            {
                let pos = Position::new(c, Direction::XY);
                let race = match self.rng.random_range(0..(1 + self.level / 2)) {
                    0 => Race::Rat,
                    1 => Race::Goblin,
                    _ => Race::Troll,
                };
                self.actors.insert(c, Actor::new(race, pos));
            }
        }

        if self.rng.random_bool(1.0 / 2.0) {
            self.items
                .insert(coord, item::random(self.level as i32, &mut self.rng));
        }
    }

    pub fn endpoint_push(&mut self, pos: h2d::Position) {
        assert!(self.map.contains_key(&pos.coord));
        self.endpoints.push_back(pos);
    }

    pub fn generate_map(mut self, start: h2d::Coordinate, size: u32) -> (Map, Actors, Items) {
        let start_dir = h2d::Direction::XY;
        let start_pos = Position::new(start, start_dir);
        let first_room_r = self.rng.random_range(2..4);
        self.start = Some(start);

        self.generate_room_inplace(start_pos, first_room_r);

        self.endpoint_push(start_pos);

        while self.tile_count < size || self.stairs.is_none() {
            let pos = self
                .endpoints
                .pop_front()
                .expect("generator run out of endpoints");

            if self.endpoints.len() > 4 {
                self.endpoints.pop_front();
            }

            assert!(
                self.map
                    .get(&pos.coord)
                    .expect("map generator iterator on non-existing tile")
                    .type_
                    .is_passable()
            );

            match self.rng.random_range(0..10) {
                0 => match self.rng.random_range(0..4) {
                    0 => self.generate_turn(pos, Left),
                    1 => self.generate_turn(pos, Right),
                    2 => self.generate_cross(pos, Left),
                    3 => self.generate_cross(pos, Right),
                    _ => panic!(),
                },
                1 => {
                    let size = self.rng.random_range(0..3) + self.rng.random_range(0..2) + 2;
                    self.generate_room(pos, size)
                }
                _ => self.generate_continue_corridor(pos),
            }
        }

        let mut map = Map::default();

        for (&coord, tile) in &self.map {
            map[coord] = *tile
        }

        // Elimination is sequential: earlier removals can expose later dead ends.
        // Fix that dependency to coordinate order rather than hash-table order.
        let mut tiles: Vec<_> = self.map.iter().collect();
        tiles.sort_by_key(|(coord, _)| (coord.x, coord.y));
        for (&coord, tile) in tiles {
            if tile.feature == Some(tile::Door(false)) && tile_is_deadend(&map, coord) {
                map[coord] = tile::Tile::new(tile::Wall);
            }
        }

        (map, self.actors, self.items)
    }
}

pub fn gen_level(level: u32, seed: u64) -> (Map, Actors, Items) {
    DungeonGenerator::new(level, seed).generate_map(Coordinate::new(0, 0), 25 + level * 50)
}
