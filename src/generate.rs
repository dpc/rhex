
use rand;
use rand::Rng;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::Arc;

use hex2d as h2d;
use hex2d::{ToCoordinate, Direction, Position, Coordinate};
use game::tile;
use game::{Map, Actors, Items};
use game::area;
use actor;

pub struct DungeonGenerator;

type EndpointQueue = VecDeque<(h2d::Coordinate, h2d::Direction)>;

fn tile_is_deadend(map : &Map, coord : Coordinate) -> bool {
    let neighbors = coord.neighbors();

    let passable : Vec<bool> = neighbors.iter().map(
        |n_coord| map.get(n_coord).map_or(false, |t| t.is_passable())
        ).collect();

    let len = passable.len();

    assert_eq!(len, 6);

    let mut changes = 0;
    let mut last = passable[len - 1];
    for i in (0..len) {
        let current = passable[i];
        if last != current {
            changes += 1;
            last = current;
        }
    }

    if changes < 4 {
        true
    } else {
        false
    }
}

impl DungeonGenerator {
    /* generate_map_feature */
   // fn generate_continue_coridor(&self, map : &mut HashMap<h2d::Coordinate, Tile>,
    fn generate_continue_coridor(&self, map : &mut Map,
                        endpoints : &mut EndpointQueue,
                        pos : h2d::Coordinate, dir : h2d::Direction) -> u32 {

        let npos = pos + dir;

        match map.get(&npos).cloned() {
            Some(tile) => {
                if tile.type_.is_passable() {
                    endpoints.push_back((npos, dir));
                } else {
                    endpoints.push_back((pos, dir + h2d::Angle::Right));
                }
                0
            },
            None => {
                map.insert(npos, tile::Tile::new(tile::Empty));
                endpoints.push_back((npos, dir));
                match rand::thread_rng().gen_range(0, 19) {
                    0 => {
                        let leftwall = pos + (dir + h2d::Angle::Left);
                        let rightwall = pos + (dir + h2d::Angle::Right);

                        if !map.contains_key(&leftwall) {
                            map.insert(leftwall, tile::Tile::new(tile::Wall));
                        }
                        if !map.contains_key(&rightwall) {
                            map.insert(rightwall, tile::Tile::new(tile::Wall));
                        }
                    }
                    _ => {}
                }

                1
            }
        }
    }

    /* generate_map_feature */
    fn generate_turn(&self, map : &mut Map,
                        endpoints : &mut EndpointQueue,
                        pos : h2d::Coordinate, dir : h2d::Direction, turn : h2d::Angle) -> u32 {

        self.generate_continue_coridor(map, endpoints, pos, dir + turn)
    }

    /* generate_map_feature */
    fn generate_cross(&self, map : &mut Map,
                        endpoints : &mut EndpointQueue,
                        pos : h2d::Coordinate, dir : h2d::Direction, turn : h2d::Angle) -> u32 {

        let cdir = dir + turn;
        let cpos = pos + cdir;
        if !map.contains_key(&cpos) {
               endpoints.push_back((pos, cdir));
        }

        self.generate_continue_coridor(map, endpoints, pos, dir)
    }

    /// Generate room in front of the iterator `(pos, dir)`
    fn generate_room(&self, map : &mut Map, actors : &mut Actors,
                        endpoints : &mut EndpointQueue,
                        pos : h2d::Coordinate, dir : h2d::Direction,
                        r : u32) -> u32 {

        endpoints.push_back((pos, dir));

        let pos = pos + dir.to_coordinate().scale(r as i32);

        let ret = self.generate_room_inplace(map, actors, pos, r);

        if ret > 0 {
            match rand::thread_rng().gen_range(0, 8) {
                0 => endpoints.push_back((pos, dir + h2d::Angle::Left)),
                1 => endpoints.push_back((pos, dir + h2d::Angle::LeftBack)),
                2 => endpoints.push_back((pos, dir + h2d::Angle::Right)),
                3 => endpoints.push_back((pos, dir + h2d::Angle::RightBack)),
                _ => {},
            }
        }
        ret
    }

    /* generate_map at position `pos`; does not push back the iterator! */
    fn generate_room_inplace(&self, map : &mut Map, actors : &mut Actors,
                             coord : h2d::Coordinate, r : u32) -> u32 {

        let mut blocked = false;
        coord.for_each_in_range((r - 1) as i32, |c| {
           if let Some(_) = map.get(&c) {
               blocked = true;
           }
        });

        if blocked {
            return 0;
        }

        match rand::thread_rng().gen_range(0, 3) {
            0 => {
                let pos = Position::new(coord, Direction::XY);
                actors.insert(coord, Arc::new(
                        actor::State::new(actor::Behavior::Grue, pos)
                        ));
            },
            _ => {},
        }

        let mut tile_count = 0;

        let area = area::Area::new(coord, area::Type::Room(r));

        coord.for_each_in_range((r - 1) as i32, |c| {
           if !map.contains_key(&c) {
               tile_count += 1;
           }
           map.insert(c, tile::Tile::new_with_area(tile::Empty, area));
        });

        // TODO: Guarantee that the room is not completely closed
        coord.for_each_in_ring(r as i32, h2d::Spin::CW(h2d::Direction::XY), |c| {
            if !map.contains_key(&c) {
                match rand::thread_rng().gen_range(0, 15) {
                    0 => {
                        map.insert(c, tile::Tile::new_with_light(tile::Wall, (r + 4) as i32));
                    },
                    _ => {
                        map.insert(c, tile::Tile::new(tile::Door(false)));
                    }
                }
            }
        });

        tile_count
    }

    pub fn generate_map(&self, start : h2d::Coordinate, size : u32) -> (Map, Actors, Items) {
        let mut map = HashMap::new();
        let mut endpoints = VecDeque::new();
        let mut actors = HashMap::new();
        let items = HashMap::new();
        let start_dir = h2d::Direction::XY;
        let first_room_r = rand::thread_rng().gen_range(0, 2) + 2;

        let mut tile_count = self.generate_room_inplace(
            &mut map, &mut actors, start, first_room_r
            );

        endpoints.push_back((start, start_dir));

        while tile_count < size {

            let (pos, dir) = endpoints.pop_front().expect("missing endpoints");

            if endpoints.len() > 4 {
                endpoints.pop_front();
            }

            assert!(map.get(&pos).expect("map generator iterator not on passable field").type_.is_passable());

            tile_count += match rand::thread_rng().gen_range(0, 10) {
                0 => {
                    match rand::thread_rng().gen_range(0, 4) {
                        0 => self.generate_turn(&mut map, &mut endpoints, pos, dir, h2d::Angle::Left),
                        1 => self.generate_turn(&mut map, &mut endpoints, pos, dir, h2d::Angle::Right),
                        2 => self.generate_cross(&mut map, &mut endpoints, pos, dir, h2d::Angle::Left),
                        3 => self.generate_cross(&mut map, &mut endpoints, pos, dir, h2d::Angle::Right),
                        _ => panic!(),
                    }
                },
                1 => {
                    let size = rand::thread_rng().gen_range(0, 3) +
                    rand::thread_rng().gen_range(0, 2) + 2;
                    self.generate_room(&mut map, &mut actors, &mut endpoints, pos, dir, size)
                },
                _ => self.generate_continue_coridor(&mut map, &mut endpoints, pos, dir),
            }

        }

        // eliminate dead ends
        for (&coord, tile) in map.clone().iter() {
            if tile.type_ == tile::Door(false) {
                if tile_is_deadend(&map, coord) {
                    map.insert(coord, tile::Tile::new(tile::Wall));
                }
            }
        }

        return (map, actors, items);
    }
}
