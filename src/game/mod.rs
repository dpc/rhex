use std::collections::{HashMap, HashSet};
use std::collections::hash_state::{DefaultState};
use simplemap::SimpleMap;
use std::sync::{Arc};
use fnv::FnvHasher;

use hex2dext::algo::bfs;
use hex2d::{Coordinate, Direction, Angle, Position};
use hex2d::Angle::{Left, Right, Forward};

use actor::{self, Race, Noise};
use generate;
use hex2dext::algo;
use item::Item;
use util::random_pos;

use self::tile::{Feature};

pub mod area;
pub mod tile;
pub mod controller;

pub use self::controller::Controller;

pub type Map = SimpleMap<Coordinate, tile::Tile>;
pub type Actors = HashMap<Coordinate, actor::State, DefaultState<FnvHasher>>;
pub type Items = HashMap<Coordinate, Box<Item>>;
pub type LightMap = SimpleMap<Coordinate, u32>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    Wait,
    Turn(Angle),
    Move(Angle),
    Charge,
    Spin(Angle),
    Equip(char),
    Fire(Coordinate),
    Pick,
    Descend,
}

#[derive(Clone, Debug)]
pub struct State {
    pub actors: HashMap<u32, actor::State, DefaultState<FnvHasher>>, // id -> State
    pub actors_pos: HashMap<Coordinate, u32, DefaultState<FnvHasher>>, // coord -> id
    pub actors_dead : HashSet<u32, DefaultState<FnvHasher>>,
    pub actors_counter : u32,
    pub map : Arc<Map>,
    pub items: Items,
    pub light_map: LightMap,
    pub turn : u64,
    pub descend : bool,
    pub level : i32,
}

pub fn action_could_be_attack(action : Action) -> bool {
    match action {
        Action::Charge => true,
        Action::Move(angle) => match angle {
            Left|Right|Forward => true,
            _ => false,
        },
        _ => false,
    }
}

impl State {
    pub fn new() -> State {

        let cp = Coordinate::new(0, 0);
        let (map, gen_actors, items) = generate::DungeonGenerator::new(0).generate_map(cp, 400);

        let mut actors : HashMap<u32, actor::State, DefaultState<FnvHasher>> = Default::default();
        let mut actors_pos : HashMap<Coordinate, u32, _> = Default::default();

        let mut actors_counter = 0u32;

        for (coord, astate) in gen_actors {
            actors_pos.insert(coord, actors_counter);
            actors.insert(actors_counter, astate);
            actors_counter += 1;
        }

        let mut state = State {
            actors: actors,
            actors_pos: actors_pos,
            actors_counter: actors_counter,
            actors_dead: Default::default(),
            items: items,
            map: Arc::new(map),
            turn: 0,
            level: 0,
            descend: false,
            light_map: LightMap::new(),
        };

        state.spawn_player(random_pos(0, 0));
        //state.spawn_pony(random_pos(-1, 0));

        state
    }

    pub fn next_level(&self) -> State {
        let cp = Coordinate::new(0, 0);
        let (map, gen_actors, items) = generate::DungeonGenerator::new(self.level + 1).generate_map(cp, 400);

        let mut actors : HashMap<u32, actor::State, DefaultState<FnvHasher>> = Default::default();
        let mut actors_pos : HashMap<Coordinate, u32, _> = Default::default();

        let mut actors_counter = 0;

        for (coord, astate) in gen_actors {
            actors_pos.insert(coord, actors_counter);
            actors.insert(actors_counter, astate);
            actors_counter += 1;
        }

        let mut player = None;
        let mut pony = None;

        for (_, astate) in self.actors.iter() {
            if astate.is_player() {
                player = Some(astate.clone());
                break;
            }
        }

        for (_, astate) in self.actors.iter() {
            if astate.race == Race::Pony {
                pony = Some(astate.clone());
                break;
            }
        }

        let mut state = State {
            actors: actors,
            actors_pos: actors_pos,
            actors_counter: actors_counter,
            actors_dead: Default::default(),
            items: items,
            map: Arc::new(map),
            turn: self.turn,
            descend: false,
            level: self.level + 1,
            light_map: Default::default(),
        };

        {
            let mut player = player.unwrap();
            let pos = random_pos(0, 0);
            player.moved(self, pos);
            state.spawn(player);
        }

        if let Some(mut pony) = pony {
            let pos = random_pos(-1, 0);
            pony.moved(self, pos);
            pony.changed_level();
            state.spawn(pony);
        }

        state
    }

    pub fn recalculate_noise(&mut self) {
        for id in &self.actors_alive_ids() {
            let source_emission = self.actors[id].noise_emision;
            if source_emission > 0 {
                let source_race = self.actors[id].race;
                let source_coord = self.actors[id].pos.coord;
                source_coord.for_each_in_range(source_emission, |coord| {
                    if let Some(&target_id) = self.actors_pos.get(&coord) {
                        self.actors.get_mut(&target_id).unwrap().noise_hears(source_coord, Noise::Creature(source_race));
                    }
                });
            }
        }
    }

    pub fn actors_ids(&self) -> Vec<u32> {
        self.actors.keys().cloned().collect()
    }

    pub fn actors_alive_ids(&self) -> Vec<u32> {
        self.actors.keys().filter(|&id| !self.actors[id].is_dead()).cloned().collect()
    }

    pub fn recalculate_light_map(&mut self) {
        let mut light_map : SimpleMap<Coordinate, u32> = Default::default();

        for (pos, tile) in self.map.iter() {
            let light = tile.light;
            if light > 0 {
                algo::los::los(
                    &|coord| {
                        if coord == *pos {
                            0
                        } else {
                            self.at(coord).tile().opaqueness()
                        }
                    },
                    &mut |coord, light| {
                        if light_map[coord] < light as u32 {
                            light_map[coord] = light as u32;
                        }
                    },
                    light, *pos, Direction::all()
                    );
            }
        }

        for (_, id) in &self.actors_pos {
            let astate = &self.actors[id];
            let pos = astate.pos.coord;
            if astate.light_emision > 0 {
                algo::los::los(
                    &|coord| {
                        if coord == pos {
                            0
                        } else {
                            self.at(coord).tile().opaqueness()
                        }
                    },
                    &mut |coord, light| {
                       if light_map[coord] < light as u32 {
                           light_map[coord] = light as u32;
                       }
                    },
                    astate.light_emision as i32, pos, Direction::all()
                );
            }
        }

        self.light_map = light_map;
    }

    pub fn spawn(&mut self, mut astate : actor::State) {
        let id = self.actors_counter;
        self.actors_counter += 1;

        self.actors_pos.insert(astate.pos.coord, id);
        let pos = astate.pos;
        astate.moved(self, pos);
        self.actors.insert(id, astate);
    }

    pub fn spawn_player(&mut self, pos : Position) {
        let mut actor = actor::State::new(actor::Race::Human, pos);
        actor.set_player();
        self.spawn(actor)
    }

    pub fn act(&mut self, id : u32, action : Action) {
        let mut actor = self.actors.remove(&id).unwrap();

        if !actor.can_perform_action() {
            self.actors.insert(id, actor);
            return;
        }
        let new_pos = actor.pos_after_action(action);

        for &new_pos in &new_pos {
            let old_pos = actor.pos;

            if actor.pos == new_pos {
                // no movement
                match action {
                    Action::Pick => {
                        let head = actor.head();
                        let item = self.at_mut(head).pick_item();

                        match item {
                            Some(item) => {
                                actor.add_item(item);
                            },
                            None => {},
                        }
                    },
                    Action::Equip(ch) => {
                        actor.equip_switch(ch);
                    },
                    Action::Descend => {
                        if self.at(actor.coord()).tile().feature == Some(Feature::Stairs) {
                            self.descend = true;
                        }
                    },
                    _ => {}
                }
            } else if action_could_be_attack(action) &&
                old_pos.coord != new_pos.coord &&
                    self.actors_pos.contains_key(&new_pos.coord)
                {
                    // we've tried to move into actor; attack?
                    if !actor.can_attack() {
                        return;
                    }
                    let dir = match action {
                        Action::Move(dir) => old_pos.dir + dir,
                        _ => old_pos.dir,
                    };

                    let target_id = self.actors_pos[&new_pos.coord];

                    let mut target = self.actors.remove(&target_id).unwrap();
                    actor.attacks(dir, &mut target);
                    self.actors.insert(target_id, target);
                    // Can't attack twice
                    break;
                } else if self.at(new_pos.coord).tile().feature == Some(tile::Door(false)
                    ) {
                    // walked into door: open it
                    let mut map = self.map.clone();
                    let mut map = Arc::make_mut(&mut map);
                    let tile = map[new_pos.coord].clone();
                    map[new_pos.coord] = tile.add_feature(tile::Door(true));
                    self.map = Arc::new(map.clone());
                    // Can't charge through the doors
                    break;
                } else if old_pos.coord == new_pos.coord || self.at(new_pos.coord).is_passable() {
                    // we've moved
                    actor.moved(self, new_pos);
                    // we will remove the previous position on post_tick, so that
                    // for the rest of this turn this actor can be found through both new
                    // and old coor
                    self.actors_pos.insert(new_pos.coord, id);
                } else {
                    // we hit the wall or something
                }
        }
        self.actors.insert(id, actor);
    }

    pub fn pre_tick(&mut self) {
        for id in self.actors_alive_ids() {
            let mut actor = self.actors.remove(&id).unwrap();
            actor.pre_tick(self);
            self.actors.insert(id, actor);
        }
    }

    /// Advance one turn (increase the turn counter) and do some maintenance
    pub fn post_tick(&mut self) {

        for id in &self.actors_ids() {
            if self.actors[id].is_dead() && !self.actors_dead.contains(&id){
                let mut a = self.actors.remove(&id).unwrap();

                for (_, item) in a.items_backpack.iter() {
                    self.at_mut(a.pos.coord).drop_item(item.clone());
                }
                a.items_backpack.clear();

                for (_, &(_, ref item)) in a.items_equipped.iter() {
                    self.at_mut(a.pos.coord).drop_item(item.clone());
                }
                a.items_equipped.clear();

                self.actors.insert(*id, a);

                self.actors_dead.insert(*id);
            }
        }

        self.actors_pos = self.actors_pos.iter().filter(|&(coord, ref id)|
                                                 !self.actors[*id].is_dead() && (self.actors[*id].coord() == *coord)
                                                ).map(|(coord, id)| (*coord, *id)).collect();

        self.recalculate_light_map();
        self.recalculate_noise();

        for id in self.actors_alive_ids() {
            let mut actor = self.actors.remove(&id).unwrap();
            actor.post_tick(self);
            self.actors.insert(id, actor);
        }

        self.turn += 1;
    }

    pub fn at(&self, coord: Coordinate) -> At {
        At {
            coord: coord,
            state: self
        }
    }

    pub fn at_mut(&mut self, coord: Coordinate) -> AtMut {
        AtMut {
            coord: coord,
            state: self
        }
    }


}

pub struct At<'a> {
    coord : Coordinate,
    state : &'a State,
}

impl<'a> At<'a> {
    // TODO: remove option
    pub fn tile(&self) -> &'a tile::Tile {
        &self.state.map[self.coord]
    }

    pub fn actor_map_or<R, F : Fn(&actor::State) -> R>
        (&self, def: R, cond : F) -> R
    {
        self.state.actors_pos.get(&self.coord).map(|&id| &self.state.actors[&id]).map_or(def, |a| cond(&a))
    }

    pub fn item_map_or<R, F : Fn(&Box<Item>) -> R>
        (&self, def: R, cond : F) -> R
    {
        self.state.items.get(&self.coord).map_or(def, |i| cond(i))
    }

    pub fn is_occupied(&self) -> bool {
        self.state.actors_pos.contains_key(&self.coord)
    }

    pub fn is_passable(&self) -> bool {
        !self.is_occupied() && self.tile().is_passable()
    }

    pub fn _light(&self) -> u32 {
        self.state.light_map[self.coord]
    }

    pub fn light_as_seen_by(&self, astate : &actor::State) -> u32 {
        let pl_coord = astate.pos.coord;

        let ownlight = self.state.light_map[self.coord];
        if self.state.map[self.coord].opaqueness() < 20 {
            ownlight
        } else {
            let reldir = -pl_coord.direction_to_cw(self.coord).unwrap_or(astate.pos.dir);
            [reldir, reldir + Left, reldir + Right].iter()
                .map(|&dir| self.coord + dir)
                .map(|d_coord|
                     if self.state.map[d_coord].opaqueness() < 20 {
                         self.state.light_map[d_coord]
                     } else {
                         0
                     })
            .max().unwrap_or(0)
        }
    }

    pub fn item(&self) -> Option<&'a Item> {
        self.state.items.get(&self.coord).map(|i| &**i)
    }
}

pub struct AtMut<'a> {
    coord : Coordinate,
    state : &'a mut State,
}

impl<'a> AtMut<'a> {
    /*
    pub fn to_at(&'a self) -> At<'a> {
        At {
            coord: self.coord,
            state: self.state
        }
    }*/

    pub fn drop_item(&mut self, item : Box<Item>) {
        let coord = {
            let mut bfs = bfs::Traverser::new(
                |coord| self.state.at(coord).tile().is_passable(),
                |coord| self.state.at(coord).tile().is_passable() && self.state.items.get(&coord).is_none(),
                self.coord
                );

            bfs.find()
        };

        match coord {
            None => { /* destroy the item :/ */ },
            Some(coord) => {
                self.state.items.insert(coord, item);
            }
        }
    }

    pub fn pick_item(&mut self) -> Option<Box<Item>> {
        if self.state.items.get(&self.coord).is_some() {
            self.state.items.remove(&self.coord)
        } else {
            None
        }
    }
}
