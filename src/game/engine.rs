use super::actor::{self, Actor};
use super::level::{ExitLink, LevelId};
use super::tile;
use super::{Action, Location};
use crate::ai::{self, Ai};
use crate::util;
use hex2d::Coordinate;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use std::collections::{HashSet, VecDeque};

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum State {
    PlayerMove,
    AiMove,
    TurnDone,
}

pub struct Engine {
    /// Stream for level seeds, initial facing, and AI decisions.
    rng: SmallRng,
    turn: u64,
    location_cur: usize,
    locations: Vec<Location>,

    ids_to_move: Vec<actor::Id>,
    state: State,
}

impl Engine {
    /// Build a headless test engine without generating a dungeon.
    #[cfg(test)]
    pub(crate) fn fixture(location: Location) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(0),
            turn: 0,
            location_cur: 0,
            locations: vec![location],
            ids_to_move: vec![],
            state: State::TurnDone,
        }
    }

    pub fn new() -> Self {
        Self::seeded(rand::rng().random())
    }

    /// Construct a repeatable game within the current RNG implementation.
    pub fn seeded(seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let location = Location::new(0, rng.random());
        Engine {
            rng,
            location_cur: 0,
            locations: vec![location],
            ids_to_move: vec![],
            turn: 0,
            state: State::TurnDone,
        }
    }

    pub fn current_location(&self) -> &Location {
        &self.locations[self.location_cur]
    }

    pub fn current_location_mut(&mut self) -> &mut Location {
        &mut self.locations[self.location_cur]
    }

    // TODO: Move field to engine
    pub fn turn(&self) -> u64 {
        self.turn
    }

    pub fn initial_spawn(&mut self, race: actor::Race) {
        let pos = util::random_pos(0, 0, &mut self.rng);
        let mut player = Actor::new(race, pos);
        player.set_player();

        self.current_location_mut().spawn_player(player);
    }

    pub fn needs_player_input(&self) -> bool {
        self.state == State::PlayerMove && self.player().can_act() && !self.player().is_dead()
    }

    pub fn player(&self) -> &Actor {
        self.current_location().player()
    }

    pub fn checks_after_act(&mut self, actor_id: actor::Id) {
        if actor_id == self.current_location().player_id()
            && self.current_location().player().descended()
        {
            if self.traverse_exit() {
                self.end_turn();
            } else {
                self.finish_action();
            }
        } else {
            self.finish_action();
        }
    }

    fn finish_action(&mut self) {
        self.state = State::AiMove;
        if self.ids_to_move.is_empty() {
            self.end_turn();
        }
    }

    /// Follow an explicit link, creating a reciprocal pair on first exploration.
    /// A crowded arrival searches only its walkable component; if full, stay put.
    fn traverse_exit(&mut self) -> bool {
        let source = self.player().coord();
        let link = if let Some(&link) = self.current_location().exits.get(&source) {
            link
        } else {
            let arrival = Coordinate::new(0, 0);
            let destination = LevelId(self.locations.len());
            let depth = self.current_location().level as u32 + 1;
            let biome = self.current_location().biome.next();
            let mut location = Location::generated(destination, depth, biome, self.rng.random());
            location.map[arrival].feature = Some(tile::Entrance);
            location.exits.insert(
                arrival,
                ExitLink {
                    destination: self.current_location().id,
                    arrival: source,
                },
            );
            self.locations.push(location);
            let link = ExitLink {
                destination,
                arrival,
            };
            self.current_location_mut().exits.insert(source, link);
            link
        };
        let Some(arrival) = Self::free_arrival(&self.locations[link.destination.0], link.arrival)
        else {
            return false;
        };
        let id = self.current_location().player_id();
        let mut player = self.current_location_mut().remove(id).unwrap();
        let old = self.current_location_mut();
        old.explored = std::mem::take(&mut player.known);
        old.explored_areas = std::mem::take(&mut player.known_areas);
        old.combat_events.clear();
        old.sound_events.clear();
        old.messages.clear();
        old.awareness_events.clear();
        self.location_cur = link.destination.0;
        let destination = self.current_location_mut();
        destination.combat_events.clear();
        destination.sound_events.clear();
        destination.messages.clear();
        destination.awareness_events.clear();
        player.pos.coord = arrival;
        destination.spawn_player(player);
        true
    }

    fn free_arrival(location: &Location, start: Coordinate) -> Option<Coordinate> {
        let mut visited = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(c) = queue.pop_front() {
            if !location.map[c].is_passable() || location.map[c].feature == Some(tile::Door(false))
            {
                continue;
            }
            if !location.actors_coord_to_id.contains_key(&c) {
                return Some(c);
            }
            for neighbor in c.neighbors() {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
        None
    }

    fn reload_actors_ids_to_move(&mut self) {
        self.ids_to_move.clear();
        let current_location = &self.locations[self.location_cur];
        let player_id = current_location.player_id();
        self.ids_to_move.extend(
            current_location
                .actors_alive_ids()
                .iter()
                .filter(|&&id| id != player_id)
                .cloned(),
        );
        // Pop in ascending spawn-ID order, after the player.
        self.ids_to_move.reverse();
    }

    // player first move
    pub fn player_act(&mut self, action: Action) {
        //assert!(self.state == State::PlayerMove); // redundant; in next assert
        assert!(self.needs_player_input());

        let player_id = self.current_location().player_id();

        self.current_location_mut().act(player_id, action);

        self.checks_after_act(player_id);
    }

    pub fn player_skip_act(&mut self) {
        assert!(self.state == State::PlayerMove);
        assert!(!self.needs_player_input());
        let player_id = self.current_location().player_id();
        self.current_location_mut().skip_act(player_id);

        self.checks_after_act(player_id);
    }

    // then everybody else one by one
    pub fn one_actor_tick(&mut self) -> actor::Id {
        assert!(self.state == State::AiMove);
        assert!(!self.needs_player_input());

        let actor_id = self.ids_to_move.pop().unwrap();

        let player_id = self.current_location().player_id();
        assert!(actor_id != player_id);

        let actor = &self.current_location().actors_byid[&actor_id].clone();
        if actor.can_act() {
            let mut ai = ai::Simple;
            let action = ai.action(actor_id, &self.locations[self.location_cur], &mut self.rng);
            self.current_location_mut().act(actor_id, action);
        } else {
            self.current_location_mut().skip_act(actor_id);
        }

        self.checks_after_act(actor_id);

        actor_id
    }

    fn end_turn(&mut self) {
        // Traversal invalidates every pending ID from the old level.
        self.ids_to_move.clear();
        self.current_location_mut().post_turn();
        self.state = State::TurnDone;
    }

    pub fn is_turn_done(&self) -> bool {
        self.state == State::TurnDone
    }

    pub fn start_turn(&mut self) {
        assert!(self.state == State::TurnDone);
        self.turn += 1;
        self.reload_actors_ids_to_move();
        self.state = State::PlayerMove;
    }
}
