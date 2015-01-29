
use rand;
use rand::Rng;
use std::collections::ring_buf::RingBuf;
use std::collections::HashMap;

use hex2d as h2d;
use hex2d::ToCoordinate;
use game::tile;
use game::Map;
use game::area;

pub struct DungeonGenerator;

type EndpointQueue = RingBuf<(h2d::Coordinate, h2d::Direction)>;

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
    fn generate_room(&self, map : &mut Map,
                        endpoints : &mut EndpointQueue,
                        pos : h2d::Coordinate, dir : h2d::Direction,
                        r : u32) -> u32 {

        endpoints.push_back((pos, dir));

        let pos = pos + dir.to_coordinate().scale(r as i32);

        let ret = self.generate_room_inplace(map, pos, r);

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
    fn generate_room_inplace(&self, map : &mut Map, pos : h2d::Coordinate, r : u32) -> u32 {

        let mut blocked = false;
        pos.for_each_in_range((r - 1) as i32, |c| {
           if let Some(_) = map.get(&c) {
               blocked = true;
           }
        });

        if blocked {
            return 0;
        }

        let mut tile_count = 0;

        let area = area::Area::new(pos, area::Type::Room(r));

        pos.for_each_in_range((r - 1) as i32, |c| {
           if !map.contains_key(&c) {
               tile_count += 1;
           }
           map.insert(c, tile::Tile::new_with_area(tile::Empty, area));
        });

        // TODO: Guarantee that the room is not completely closed
        pos.for_each_in_ring(r as i32, h2d::Spin::CW(h2d::Direction::XY), |c| {
            match rand::thread_rng().gen_range(0, 2) {
                0 => {
                    if !map.contains_key(&c) {
                        match rand::thread_rng().gen_range(0, 15) {
                            0 => {
                                map.insert(c, tile::Tile::new_with_light(tile::Wall, (r + 4) as i32));
                            },
                            _ => {
                                map.insert(c, tile::Tile::new(tile::Wall));
                            }
                        }
                    }
                },
                _ => {}
            }
        });

        tile_count
    }

    pub fn generate_map(&self, start : h2d::Coordinate, size : u32) -> Map {
        let mut map = HashMap::new();
        let mut endpoints = RingBuf::new();

        let start_dir = h2d::Direction::XY;
        let first_room_r = rand::thread_rng().gen_range(0, 2) + 2;

        let mut tile_count = self.generate_room_inplace(
            &mut map, start, first_room_r
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
                    self.generate_room(&mut map, &mut endpoints, pos, dir, size)
                },
                _ => self.generate_continue_coridor(&mut map, &mut endpoints, pos, dir),
            }

        }

        return map;
    }
}
