use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc};

use hex2d::{Coordinate, Direction, Angle};
use actor;
use generate;
use hex2dext::algo;

pub mod area;
pub mod tile;
pub mod controller;

pub use self::controller::Controller;

pub type Map = HashMap<Coordinate, tile::Tile>;
pub type Actors = HashMap<Coordinate, Arc<actor::State>>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    Wait,
    Turn(Angle),
    Move(Angle),
    Spin(Angle),
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub actors: Arc<Actors>,
    pub map : Arc<Map>,
    pub light_map: Arc<HashMap<Coordinate, u32>>,
    pub turn : u64,
}

impl State {
    pub fn new() -> State {

        let cp = Coordinate::new(0, 0);
        let (map, actors) = generate::DungeonGenerator.generate_map(cp, 400);

        let mut state = State {
            actors: Arc::new(actors),
            map: Arc::new(map),
            turn: 0,
            light_map: Arc::new(HashMap::new()),
        };

        state.recalculate_light_map();

        state
    }

    pub fn recalculate_light_map(&mut self) {
        let mut light_map : HashMap<Coordinate, u32> = HashMap::new();

        for (pos, tile) in &*self.map {
            let light = tile.light;
            if light > 0 {
                algo::los::los(
                    &|coord| {
                        if coord == *pos {
                            0
                        } else {
                            self.tile_at(coord).map_or(light, |tile| tile.opaqueness())
                        }
                    },
                    &mut |coord, light| {
                        match light_map.entry(coord) {
                            Entry::Occupied(mut entry) => {
                                let val = entry.get_mut();
                                if light as u32 > *val {
                                    *val = light as u32;
                                }
                            },
                            Entry::Vacant(entry) => {
                                entry.insert(light as u32);
                            },
                        }
                    },
                    light, *pos, Direction::all()
                    );
            }
        }

        for (pos, astate) in &*self.actors {
            if astate.light > 0 {
                algo::los::los(
                    &|coord| {
                        if coord == *pos {
                            0
                        } else {
                            self.tile_at(coord).map_or(astate.light as i32, |tile| tile.opaqueness())
                        }
                    },
                    &mut |coord, light| {
                        match light_map.entry(coord) {
                            Entry::Occupied(mut entry) => {
                                let val = entry.get_mut();
                                if light as u32 > *val {
                                    *val = light as u32;
                                }
                            },
                            Entry::Vacant(entry) => {
                                entry.insert(light as u32);
                            },
                        }
                    },
                    astate.light as i32, *pos, Direction::all()
                    );
            }
        }

        self.light_map = Arc::new(light_map);
    }

    pub fn spawn(&self, pos : Coordinate, behavior : actor::Behavior, light : u32) -> State {

        let mut actors = self.actors.clone().make_unique().clone();

        actors.insert(pos, Arc::new(actor::State::new(behavior, pos, Direction::XY, self).add_light(light))
);

        State {
            actors: Arc::new(actors),
            map: self.map.clone(),
            turn: self.turn,
            light_map: self.light_map.clone(),
        }
    }

    pub fn spawn_player(&self) -> State {
        self.spawn(Coordinate::new(0, 0), actor::Behavior::Player, 0)
    }

    pub fn spawn_monster(&self) -> State {
        self.spawn(Coordinate::new(0, 1), actor::Behavior::Grue, 0)
    }

    pub fn spawn_pony(&self, pos : Coordinate) -> State {
        self.spawn(pos, actor::Behavior::Pony, 7)
    }

    pub fn act(&self, actor : &actor::State, action : Action) -> State {
        let mut arc = self.actors.clone();
        let mut actors = arc.make_unique().clone();

        let new_actor_state = actor.act(self, action);
        actors.remove(&actor.pos);
        assert!(!actors.contains_key(&new_actor_state.pos));
        actors.insert(new_actor_state.pos, Arc::new(new_actor_state));

        let mut ret = State {
            actors: Arc::new(actors),
            map: self.map.clone(),
            turn: self.turn,
            light_map: Arc::new(HashMap::new()),
        };

        ret.recalculate_light_map();

        ret
    }

    /// Advance one turn (increase the turn counter)
    pub fn tick(&self) -> State {
        State {
            actors: self.actors.clone(),
            map: self.map.clone(),
            turn: self.turn + 1,
            light_map: self.light_map.clone(),
        }
    }

    pub fn actor_map_or<R, F : Fn(&actor::State) -> R>(&self, pos : Coordinate, def: R, cond : &F) -> R {
        self.actors.get(&pos).map_or(def, |a| cond(a))
    }

    pub fn tile_at(&self, pos : Coordinate) -> Option<&tile::Tile> {
        self.map.get(&pos)
    }

    pub fn tile_map<R, F : Fn(&tile::Tile) -> R>(&self, pos : Coordinate, f : F) -> Option<R> {
        self.map.get(&pos).map(|a| f(a))
    }

    pub fn tile_map_or<R, F : Fn(&tile::Tile) -> R>(&self, pos : Coordinate, def: R, f : F) -> R {
        self.map.get(&pos).map_or(def, |a| f(a))
    }

    pub fn occupied(&self, pos : Coordinate) -> bool {
        self.actors.contains_key(&pos)
    }

    pub fn passable(&self, pos : Coordinate) -> bool {
        !self.occupied(pos) && self.tile_map_or(pos, false, |t| t.is_passable())
    }

    pub fn light(&self, pos : Coordinate) -> u32 {
        self.light_map.get(&pos).map_or(0, |l| *l)
    }
}

