use super::{Location, Action};
use super::actor::{self, Actor};
use util;
use ai::{self, Ai};

pub struct Engine {
    turn : u64,
    location_cur : usize,
    locations : Vec<Location>,

    ids_to_move : Vec<actor::Id>,
}

impl Engine {
    pub fn new() -> Self {
        let location = Location::new(0);
        Engine {
            location_cur : 0,
            locations: vec!(location),
            ids_to_move: vec!(),
            turn: 0,
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

    pub fn initial_spawn(&mut self) {
        let pos = util::random_pos(0, 0);
        let mut player = Actor::new(actor::Race::Human, pos);
        player.set_player();

        self.current_location_mut().spawn_player(player);
    }

    pub fn needs_player_input(&self) -> bool {
        self.ids_to_move.is_empty()
    }

    pub fn checks_after_act(&mut self, actor_id : actor::Id) {
        if actor_id == self.current_location().player_id() &&
            self.current_location().player().descended() {
            let mut player = self.current_location_mut().remove(actor_id).unwrap();
            self.location_cur += 1;
            self.locations.push(Location::new(self.location_cur as u32));
            player.pos = util::random_pos(0, 0);
            let player = self.current_location_mut().spawn_player(player);
        }

        if self.ids_to_move.is_empty() {
            self.end_turn();
            let player_id = self.current_location().player_id();
            let player = &self.current_location().actors_byid[&player_id].clone();
            if !player.can_act() {
                self.current_location_mut().skip_act(player_id);
                self.reload_actors_ids_to_move();
            }
        }
    }

    pub fn reload_actors_ids_to_move(&mut self) {
        let player_id = self.current_location().player_id();
        self.ids_to_move = self.current_location().actors_alive_ids()
            .iter()
            .cloned()
            .filter(|&id| id != player_id).collect();
    }

    // player first move
    pub fn player_act(&mut self, action : Action) {
        assert!(self.needs_player_input());

        let player_id = self.current_location().player_id();

        self.current_location_mut().act(player_id, action);

        self.reload_actors_ids_to_move();

        self.checks_after_act(player_id);
    }

    // then everybody else one by one
    pub fn one_actor_tick(&mut self) -> actor::Id {
        assert!(!self.needs_player_input());

        let actor_id = self.ids_to_move.pop().unwrap();

        let player_id = self.current_location().player_id();
        assert!(actor_id != player_id);

        let actor = &self.current_location().actors_byid[&actor_id].clone();
        if actor.can_act() {
            let mut ai = ai::Simple;
            let action = ai.action(actor_id, self);
            self.current_location_mut().act(actor_id, action);
        } else {
            self.current_location_mut().skip_act(actor_id);
        }

        self.checks_after_act(actor_id);

        actor_id
    }

    pub fn end_turn(&mut self) {
        self.turn += 1;
        self.current_location_mut().post_turn()
    }
}
