use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::Entry;
use std::sync::{Arc};

use hex2d::{Coordinate, Direction, Angle, Position};
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Stage {
    ST1,
    ST2,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct State {
    pub actors: Arc<Actors>,
    pub actors_done: Arc<HashSet<Coordinate>>,
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
            actors_done: Arc::new(HashSet::new()),
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

    pub fn spawn(&self, coord : Coordinate, behavior : actor::Behavior, light : u32) -> State {

        let mut actors = self.actors.clone().make_unique().clone();

        let pos = Position::new(coord, Direction::XY);

        actors.insert(pos.coord, Arc::new(
                actor::State::new(behavior, pos, self).add_light(light)
                ));

        State {
            actors: Arc::new(actors),
            actors_done: self.actors_done.clone(),
            map: self.map.clone(),
            turn: self.turn,
            light_map: self.light_map.clone(),
        }
    }

    pub fn spawn_player(&self) -> State {
        self.spawn(Coordinate::new(0, 0), actor::Behavior::Player, 0)
    }

    pub fn spawn_pony(&self, pos : Coordinate) -> State {
        self.spawn(pos, actor::Behavior::Pony, 7)
    }

    pub fn actor_act(&self, stage : Stage,
                     astate : &actor::State,
                     action : Action) -> Option<State> {
        let new_pos = astate.pos_after_action(action);

        if astate.pos == new_pos {
            // we did nothing
            None
        } else if astate.pos.coord != new_pos.coord && self.actors.contains_key(&new_pos.coord) {
            // that was an attack!
            if stage != Stage::ST1 {
                return None;
            }
            let mut actors = self.actors.clone().make_unique().clone();
            let mut actors_done = self.actors_done.clone().make_unique().clone();

            let target = &self.actors[new_pos.coord];
            let target_new_state = target.hit();
            actors.remove(&new_pos.coord);
            actors.insert(target_new_state.pos.coord, Arc::new(target_new_state));

            actors_done.insert(new_pos.coord);

            let ret = State {
                actors: Arc::new(actors),
                actors_done: Arc::new(actors_done),
                map: self.map.clone(),
                turn: self.turn,
                light_map: Arc::new(HashMap::new()),
            };
            Some(ret)
        } else if astate.pos.coord == new_pos.coord || self.is_passable(new_pos.coord) {
            // we've moved
            if stage != Stage::ST2 {
                return None
            }

            let mut actors = self.actors.clone().make_unique().clone();
            let actor_new_state = astate.change_position(new_pos, self);

            actors.remove(&astate.pos.coord);
            actors.insert(actor_new_state.pos.coord, Arc::new(actor_new_state));

            let ret = State {
                actors: Arc::new(actors),
                actors_done: self.actors_done.clone(),
                map: self.map.clone(),
                turn: self.turn,
                light_map: Arc::new(HashMap::new()),
            };
            Some(ret)
        } else {
            // we hit the wall or something
            None
        }
    }

    pub fn act(&self, stage : Stage,
               astate : &actor::State, action : Action) -> State {

        if self.actors_done.contains(&astate.pos.coord) {
            return self.clone()
        }

        if let Some(state) = self.actor_act(stage, astate, action) {
            state
        } else {
            self.clone()
        }
    }

    /// Advance one turn (increase the turn counter) and do some maintenance
    pub fn tick(&self) -> State {

        // filter out the dead
        // TODO: Make this work
        // let actors = self.actors.make_unique().iter().filter(|&(ref coord, ref a)| a.stats.hp > 0).cloned().collect();
        let mut actors = HashMap::new();

        for (&coord, a) in self.actors.iter() {
            if a.stats.hp > 0 {
                actors.insert(coord, a.clone());
            }
        }

        let mut ret = State {
            actors: Arc::new(actors),
            actors_done: Arc::new(HashSet::new()),
            map: self.map.clone(),
            turn: self.turn + 1,
            light_map: self.light_map.clone(),
        };
        ret.recalculate_light_map();
        ret
    }

    pub fn actor_map_or<R, F : Fn(&actor::State) -> R>
        (&self, pos : Coordinate, def: R, cond : &F) -> R
    {
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

    pub fn is_occupied(&self, pos : Coordinate) -> bool {
        self.actors.contains_key(&pos)
    }

    pub fn is_passable(&self, pos : Coordinate) -> bool {
        !self.is_occupied(pos) && self.tile_map_or(pos, false, |t| t.is_passable())
    }

    pub fn light(&self, pos : Coordinate) -> u32 {
        self.light_map.get(&pos).map_or(0, |l| *l)
    }
}

