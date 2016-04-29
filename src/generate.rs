
use rand;
use rand::{XorShiftRng, Rng, SeedableRng};
use std::collections::{HashMap, VecDeque};
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;

use hex2d as h2d;
use hex2d::Angle::*;
use hex2d::{ToCoordinate, Direction, Position, Coordinate};
use game::tile;
use game::{Map, Actors, Items};
use game::area;
use game::item;
use game::actor::{Race, Actor};

type EndpointQueue = VecDeque<h2d::Position>;

pub struct DungeonGenerator {
    level: u32,
    start: Option<Coordinate>,
    stairs: Option<Coordinate>,
    tile_count: u32,
    map: HashMap<Coordinate, tile::Tile, BuildHasherDefault<FnvHasher>>,
    endpoints: EndpointQueue,
    actors: Actors,
    items: Items,
    rng: XorShiftRng
}

impl DungeonGenerator {
    pub fn new(level: u32) -> DungeonGenerator {
        DungeonGenerator {
            level: level,
            start: None,
            stairs: None,
            tile_count: 0,
            map: Default::default(),
            endpoints: VecDeque::new(),
            actors: Default::default(),
            items: Default::default(),
            rng: XorShiftRng::from_seed(rand::thread_rng().gen::<[u32; 4]>()),
        }
    }
}

fn tile_is_deadend(map: &Map, coord: Coordinate) -> bool {
    let neighbors = coord.neighbors();

    let passable: Vec<bool> = neighbors.iter()
                                       .map(|n_coord| map[*n_coord].is_passable())
                                       .collect();

    let len = passable.len();

    assert_eq!(len, 6);

    let mut changes = 0;
    let mut last = passable[len - 1];
    for i in 0..len {
        let current = passable[i];
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

        let npos = pos + pos.dir.to_coordinate();

        let somepos = if let Some(tile) = self.map.get(&npos.coord) {
            Some(if tile.type_.is_passable() { npos } else { pos + Right })
        } else { None };
        match somepos {
            Some(pos) => self.endpoint_push(pos),
            None => {
                self.map.insert(npos.coord, tile::Tile::new(tile::Empty));
                self.endpoint_push(npos);
                match self.rng.gen_range(0, 19) {
                    0 => {
                        let leftwall = pos + (pos.dir + h2d::Angle::Left).to_coordinate();
                        let rightwall = pos + (pos.dir + h2d::Angle::Right).to_coordinate();

                        self.map.entry(leftwall.coord).or_insert(tile::Tile::new(tile::Wall));
                        self.map.entry(rightwall.coord).or_insert(tile::Tile::new(tile::Wall));
                    }
                    _ => {}
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

        let center_pos = pos + pos.dir.to_coordinate().scale(r as i32);

        let tile_count_old = self.tile_count;
        self.generate_room_inplace(center_pos, r);

        if tile_count_old == self.tile_count {
            match self.rng.gen_range(0, 8) {
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
        coord.for_each_in_range((r - 1) as i32, |c| {
            if self.map.contains_key(&c) {
                blocked = true;
            }
        });

        if blocked {
            return;
        }

        let area = area::Area::new(coord, area::Type::Room(r));

        if Some(coord) != self.start {
            match self.rng.gen_range(0, 6) {
                2 => {
                    if self.stairs.is_none() {
                        self.map.insert(coord,
                                        *tile::Tile::new(tile::Empty)
                                             .add_feature(tile::Stairs)
                                             .add_area(area));
                        self.stairs = Some(coord);
                        self.tile_count += 1;
                    }
                }
                3 => {
                    self.map.insert(coord,
                                    *tile::Tile::new(tile::Empty)
                                         .add_feature(tile::Statue)
                                         .add_area(area));
                    self.tile_count += 1;
                }
                _ => {}
            }
        }

        coord.for_each_in_range((r - 1) as i32, |c| {
            if !self.map.contains_key(&c) {
                self.tile_count += 1;
                self.map.insert(c, *tile::Tile::new(tile::Empty).add_area(area));
            }
        });

        // TODO: Guarantee that the room is not completely closed
        coord.for_each_in_ring(r as i32, h2d::Spin::CW(h2d::Direction::XY), |c| {
            if !self.map.contains_key(&c) {
                self.tile_count += 1;
                self.map.insert(c,
                                *tile::Tile::new(tile::Empty).add_feature(tile::Door(false)));
            }
        });

        coord.for_each_in_range(r as i32 - 1, |c| {
            if self.map.contains_key(&c) {
                match self.rng.gen_range(0, 15) {
                    0 => {
                        self.map.get_mut(&c).unwrap().add_light((r + 4) as i32);
                    }
                    _ => {}
                }
                self.tile_count += 1;
            }
        });


        coord.for_each_in_range(r as i32 / 2, |c| {
            if c != coord && self.map.get(&c).map_or(false, |t| t.is_passable()) {
                match self.rng.gen_range(0, 10) {
                    0 => {
                        let pos = Position::new(c, Direction::XY);
                        let race = match self.rng.gen_range(0, 1 + self.level / 2) {
                            0 => Race::Rat,
                            1 => Race::Goblin,
                            _ => Race::Troll,
                        };
                        self.actors.insert(c, Actor::new(race, pos));
                    }
                    _ => {}
                }
            }
        });

        if self.rng.gen_weighted_bool(2) {
            self.items.insert(coord, item::random(self.level as i32));
        }
    }

    pub fn endpoint_push(&mut self, pos: h2d::Position) {
        assert!(self.map.contains_key(&pos.coord));
        self.endpoints.push_back(pos);
    }

    pub fn generate_map(mut self, start: h2d::Coordinate, size: u32) -> (Map, Actors, Items) {
        let start_dir = h2d::Direction::XY;
        let start_pos = Position::new(start, start_dir);
        let first_room_r = self.rng.gen_range(2, 4);
        self.start = Some(start);

        self.generate_room_inplace(start_pos, first_room_r);

        self.endpoint_push(start_pos);

        while self.tile_count < size || self.stairs.is_none() {

            let pos = self.endpoints.pop_front().expect("generator run out of endpoints");

            if self.endpoints.len() > 4 {
                self.endpoints.pop_front();
            }

            assert!(self.map
                        .get(&pos.coord)
                        .expect("map generator iterator on non-existing tile")
                        .type_
                        .is_passable());

            match self.rng.gen_range(0, 10) {
                0 => {
                    match self.rng.gen_range(0, 4) {
                        0 => self.generate_turn(pos, Left),
                        1 => self.generate_turn(pos, Right),
                        2 => self.generate_cross(pos, Left),
                        3 => self.generate_cross(pos, Right),
                        _ => panic!(),
                    }
                }
                1 => {
                    let size = self.rng.gen_range(0, 3) +
                               self.rng.gen_range(0, 2) +
                               2;
                    self.generate_room(pos, size)
                }
                _ => self.generate_continue_corridor(pos),
            }
        }

        let mut map = Map::default();

        for (&coord, tile) in &self.map {
            map[coord] = tile.clone()
        }

        // eliminate dead ends
        for (&coord, tile) in &self.map {
            if tile.feature == Some(tile::Door(false)) &&
                tile_is_deadend(&map, coord) {
                    map[coord] = tile::Tile::new(tile::Wall);
                }
        }

        (map, self.actors, self.items)
    }
}

pub fn gen_level(level: u32) -> (Map, Actors, Items) {
    DungeonGenerator::new(level).generate_map(Coordinate::new(0, 0), 25 + level * 50)
}
