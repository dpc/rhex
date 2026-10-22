use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;

use crate::generate;

use crate::hex2dext::algo;
use fnv::FnvHasher;
use hex2d::{Coordinate, Direction};
use rand::{SeedableRng, rngs::SmallRng};
use simplemap::SimpleMap;

use super::Action;
use super::actor::{self, Actor};
use super::item::Item;
use super::tile;
use super::{Items, LightMap, Map};

#[cfg(test)]
#[path = "awareness_tests.rs"]
mod awareness_tests;
#[cfg(test)]
#[path = "location_tests.rs"]
mod tests;

#[derive(Clone, Debug)]
pub struct Location {
    /// Owned tactical stream, never accessed by presentation.
    pub(crate) rng: SmallRng,
    pub actors_byid: HashMap<actor::Id, Actor>, // id -> State
    pub actors_coord_to_id: HashMap<Coordinate, u32>, // coord -> id
    pub actors_dead: HashSet<actor::Id>,
    pub actors_counter: u32,
    pub map: Map,
    pub items: Items,        // items on the floor
    pub light_map: LightMap, // light intensity at a given coordinate
    pub level: i32,
    player_id: Option<actor::Id>,
    /// Observed resolved hits awaiting one-time presentation consumption.
    pub combat_events: Vec<super::combat_event::CombatEvent>,
    /// Action-scoped observations; consumers drain these after each action.
    pub sound_events: Vec<super::sound::ObservedSound>,
    /// Visible awareness transitions, drained once or expired at the next action.
    pub awareness_events: Vec<super::awareness_event::AwarenessEvent>,
}

impl Location {
    /// Capture visible hit geometry while both participants still exist.
    pub fn record_hit(&mut self, source: &Actor, target: &Actor, success: bool) {
        let player = if source.is_player() {
            source
        } else if target.is_player() {
            target
        } else {
            self.player()
        };
        if success && player.sees(source.coord()) && player.sees(target.coord()) {
            self.combat_events.push(super::combat_event::CombatEvent {
                source: source.coord(),
                impact: target.coord(),
            });
        }
    }
    /// Build an explicitly populated test level without procedural generation.
    #[cfg(test)]
    pub(crate) fn fixture(map: Map, mut player: Actor) -> Self {
        player.recalculate_stats();
        Self {
            rng: SmallRng::seed_from_u64(0),
            actors_coord_to_id: [(player.pos.coord, 0)].into(),
            actors_byid: [(0, player)].into(),
            actors_dead: Default::default(),
            actors_counter: 1,
            map,
            items: Default::default(),
            light_map: Default::default(),
            level: 0,
            player_id: Some(0),
            combat_events: Vec::new(),
            sound_events: Vec::new(),
            awareness_events: Vec::new(),
        }
    }
    pub fn new(level: u32, seed: u64) -> Location {
        let (map, gen_actors, items) = generate::gen_level(level, seed);

        let mut actors: HashMap<u32, Actor> = Default::default();
        let mut actors_pos: HashMap<Coordinate, u32> = Default::default();

        let mut actors_counter = 0u32;

        // Coordinates assign stable actor IDs independent of hash table layout.
        let mut gen_actors: Vec<_> = gen_actors.into_iter().collect();
        gen_actors.sort_by_key(|(coord, _)| (coord.x, coord.y));
        for (coord, astate) in gen_actors {
            actors_pos.insert(coord, actors_counter);
            actors.insert(actors_counter, astate);
            actors_counter += 1;
        }

        Location {
            rng: SmallRng::seed_from_u64(seed),
            actors_byid: actors,
            actors_coord_to_id: actors_pos,
            actors_counter,
            actors_dead: Default::default(),
            items,
            map,
            level: level as i32,
            light_map: Default::default(),
            player_id: None,
            combat_events: Vec::new(),
            sound_events: Vec::new(),
            awareness_events: Vec::new(),
        }
    }

    pub fn player_id(&self) -> actor::Id {
        self.player_id.unwrap()
    }

    pub fn player(&self) -> &Actor {
        &self.actors_byid[&self.player_id.unwrap()]
    }

    /// Change current awareness; resetting permits a later acquisition event.
    pub fn set_player_awareness(&mut self, id: actor::Id, aware: bool) {
        let Some(player_id) = self.player_id else {
            return;
        };
        let player = &self.actors_byid[&player_id];
        let enemy = &self.actors_byid[&id];
        if id == player_id || enemy.is_player() || enemy.is_dead() || player.is_dead() {
            return;
        }
        let observed = player.sees(enemy.coord());
        let enemy = self.actors_byid.get_mut(&id).unwrap();
        if aware && !enemy.aware_of_player && observed {
            self.awareness_events
                .push(super::awareness_event::AwarenessEvent { actor: id });
        }
        enemy.aware_of_player = aware;
    }

    /// Acquire sight immediately after world changes, not at the next AI action.
    fn resolve_player_sight(&mut self) {
        let Some(player_id) = self.player_id else {
            return;
        };
        let player = &self.actors_byid[&player_id];
        if player.is_dead() {
            return;
        }
        let coord = player.coord();
        for id in self.actors_alive_ids() {
            let enemy = &self.actors_byid[&id];
            if id != player_id && !enemy.aware_of_player && enemy.currently_sees(self, coord) {
                self.set_player_awareness(id, true);
            }
        }
    }

    /// Resolve discrete emissions once against post-action geometry.
    pub fn resolve_sounds(&mut self) {
        for id in self.actors_ids() {
            let emissions =
                std::mem::take(&mut self.actors_byid.get_mut(&id).unwrap().pending_noises);
            for emission in emissions {
                let cells = super::sound::propagate(&self.map, &self.items, &emission);
                for cell in &cells {
                    if let Some(&listener) = self.actors_coord_to_id.get(&cell.coord) {
                        self.actors_byid
                            .get_mut(&listener)
                            .unwrap()
                            .noise_hears(emission.source, emission.kind);
                        if Some(id) == self.player_id {
                            self.set_player_awareness(listener, true);
                        }
                    }
                }
                if let Some(player_id) = self.player_id {
                    let player = &self.actors_byid[&player_id];
                    if cells.iter().any(|cell| cell.coord == player.coord()) {
                        self.sound_events.push(super::sound::ObservedSound {
                            hidden_source: (id != player_id && !player.sees(emission.source))
                                .then_some(emission.source),
                            cells: cells
                                .into_iter()
                                .filter(|cell| player.sees(cell.coord))
                                .collect(),
                        });
                    }
                }
            }
        }
    }

    pub fn actors_ids(&self) -> Vec<u32> {
        let mut ids: Vec<_> = self.actors_byid.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    pub fn actors_alive_ids(&self) -> Vec<u32> {
        self.actors_ids()
            .into_iter()
            .filter(|id| !self.actors_byid[id].is_dead())
            .collect()
    }

    pub fn recalculate_light_map(&mut self) {
        let mut light_map: SimpleMap<Coordinate, u32, BuildHasherDefault<FnvHasher>> =
            Default::default();

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
                    light,
                    *pos,
                    Direction::all(),
                );
            }
        }

        for id in self.actors_coord_to_id.values() {
            let astate = &self.actors_byid[id];
            let pos = astate.pos.coord;
            if astate.light_emision() > 0 {
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
                    astate.light_emision() as i32,
                    pos,
                    Direction::all(),
                );
            }
        }

        self.light_map = light_map;
    }

    pub fn spawn(&mut self, mut astate: Actor) -> actor::Id {
        if self.actors_coord_to_id.contains_key(&astate.pos.coord) {
            // TODO: Find an alternative place
            unimplemented!();
        }
        let id = self.actors_counter;
        self.actors_counter += 1;

        debug_assert!(!self.actors_coord_to_id.contains_key(&astate.pos.coord));
        self.actors_coord_to_id.insert(astate.pos.coord, id);
        astate.pre_spawn(self);
        self.actors_byid.insert(id, astate);
        self.recalculate_light_map();

        self.pre_any_tick();
        let mut astate = self.actors_byid.remove(&id).unwrap();
        astate.post_spawn(self);
        self.actors_byid.insert(id, astate);
        self.post_any_tick();

        id
    }

    pub fn remove(&mut self, id: actor::Id) -> Option<Actor> {
        let actor = self.actors_byid.remove(&id);

        let actor = actor?;

        self.actors_coord_to_id.remove(&actor.pos.coord);

        Some(actor)
    }

    pub fn spawn_player(&mut self, actor: Actor) -> actor::Id {
        assert!(actor.is_player());
        // Publish identity before spawn's perception boundary, including first contact.
        self.player_id = Some(self.actors_counter);
        self.spawn(actor);
        self.player_id.unwrap()
    }

    pub fn skip_act(&mut self, id: u32) {
        self.pre_any_tick();
        let mut actor = self.actors_byid.remove(&id).unwrap();
        actor.pre_own_tick();
        actor.post_own_tick(self);
        self.actors_byid.insert(id, actor);
        self.post_any_tick();
    }

    pub fn act(&mut self, id: u32, action: Action) {
        self.pre_any_tick();
        let mut actor = self.actors_byid.remove(&id).unwrap();

        if !actor.can_perform_action() {
            self.actors_byid.insert(id, actor);
            return;
        }

        actor.pre_own_tick();
        actor.act(self, action);

        actor.post_own_tick(self);
        self.actors_byid.insert(id, actor);
        self.actors_byid.get_mut(&id).unwrap().post_action(action);
        self.post_any_tick();
    }

    pub fn pre_any_tick(&mut self) {
        self.sound_events.clear();
        self.awareness_events.clear();
        for id in self.actors_alive_ids() {
            let mut actor = self.actors_byid.remove(&id).unwrap();
            actor.pre_any_tick();
            self.actors_byid.insert(id, actor);
        }
    }

    pub fn post_any_tick(&mut self) {
        for id in self.actors_alive_ids() {
            let mut actor = self.actors_byid.remove(&id).unwrap();
            actor.post_any_tick(self);
            self.actors_byid.insert(id, actor);
        }

        for id in &self.actors_ids() {
            if self.actors_byid[id].is_dead() && !self.actors_dead.contains(id) {
                let mut a = self.actors_byid.remove(id).unwrap();

                // Letter order determines which item occupies the corpse tile.
                let mut items: Vec<_> = a.items_backpack.drain().collect();
                items.extend(a.items_equipped.drain().map(|(_, entry)| entry));
                items.sort_by_key(|(letter, _)| *letter);
                for (_, item) in items {
                    self.at_mut(a.pos.coord).drop_item(item);
                }

                self.actors_byid.insert(*id, a);

                self.actors_dead.insert(*id);
            }
        }

        self.actors_coord_to_id = self
            .actors_coord_to_id
            .iter()
            .filter(|&(_, id)| !self.actors_byid[id].is_dead())
            .map(|(_, id)| (self.actors_byid[id].pos.coord, *id))
            .collect();

        debug_assert!(
            self.actors_coord_to_id
                .iter()
                .all(|(coord, id)| self.actors_byid[id].coord() == *coord)
        );

        self.recalculate_light_map();
        self.resolve_player_sight();
        self.resolve_sounds();
    }

    pub fn post_turn(&mut self) {}

    pub fn at(&self, coord: Coordinate) -> At<'_> {
        At { coord, state: self }
    }

    pub fn at_mut(&mut self, coord: Coordinate) -> AtMut<'_> {
        AtMut { coord, state: self }
    }
}

pub struct At<'a> {
    coord: Coordinate,
    state: &'a Location,
}

impl<'a> At<'a> {
    // TODO: remove option
    pub fn tile(&self) -> &'a tile::Tile {
        &self.state.map[self.coord]
    }

    pub fn actor_map_or<R, F: Fn(&Actor) -> R>(&self, def: R, cond: F) -> R {
        self.state
            .actors_coord_to_id
            .get(&self.coord)
            .map(|&id| &self.state.actors_byid[&id])
            .map_or(def, cond)
    }

    pub fn item_map_or<R, F: Fn(&Box<Item>) -> R>(&self, def: R, cond: F) -> R {
        self.state.items.get(&self.coord).map_or(def, cond)
    }

    pub fn is_occupied(&self) -> bool {
        self.state.actors_coord_to_id.contains_key(&self.coord)
    }

    pub fn is_passable(&self) -> bool {
        !self.is_occupied() && self.tile().is_passable()
    }

    pub fn _light(&self) -> u32 {
        self.state.light_map[self.coord]
    }

    pub fn light_as_seen_by(&self, astate: &Actor) -> u32 {
        let pl_coord = astate.pos.coord;

        let ownlight = self.state.light_map[self.coord];
        if self.state.map[self.coord].opaqueness() < 20 {
            ownlight
        } else {
            pl_coord
                .directions_to(self.coord)
                .iter()
                .map(|&dir| self.coord - dir)
                .map(|d_coord| {
                    if self.state.map[d_coord].opaqueness() < 20 {
                        self.state.light_map[d_coord]
                    } else {
                        0
                    }
                })
                .max()
                .unwrap_or(0)
        }
    }

    pub fn item(&self) -> Option<&'a Item> {
        self.state.items.get(&self.coord).map(|i| &**i)
    }
}

pub struct AtMut<'a> {
    coord: Coordinate,
    state: &'a mut Location,
}

impl<'a> AtMut<'a> {
    pub fn drop_item(&mut self, item: Box<Item>) {
        let coord = {
            let mut bfs = algo::bfs::Traverser::new(
                |coord| self.state.at(coord).tile().is_passable(),
                |coord| {
                    self.state.at(coord).tile().is_passable()
                        && !self.state.items.contains_key(&coord)
                },
                self.coord,
            );

            bfs.find()
        };

        match coord {
            None => {
                // destroy the item :/
            }
            Some(coord) => {
                self.state.items.insert(coord, item);
            }
        }
    }

    pub fn pick_item(&mut self) -> Option<Box<Item>> {
        if self.state.items.contains_key(&self.coord) {
            self.state.items.remove(&self.coord)
        } else {
            None
        }
    }
}
