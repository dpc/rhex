use super::{Location, Action};
use util;
use ai::{self, Ai};

pub struct Engine {
    location_cur : usize,
    locations : Vec<Location>,

    player_move : bool,
}

impl Engine {
    pub fn new() -> Self {
        let location = Location::new();
        Engine {
            location_cur : 0,
            locations: vec!(location),
            player_move: true,
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
        self.current_location().turn
    }

    pub fn spawn(&mut self) {
        self.current_location_mut().spawn_player(util::random_pos(0, 0));
    }

    pub fn needs_player_input(&self) -> bool {
        self.player_move
    }

    pub fn player_act(&mut self, action : Action) {
        assert!(self.needs_player_input());

        let player_id = self.current_location().player_id();

        self.current_location_mut().act(player_id, action);

        self.player_move = false;
    }

    pub fn tick(&mut self) {
        assert!(!self.needs_player_input());

        let player_id = self.current_location().player_id();

        for id in self.current_location().actors_alive_ids() {
            if id != player_id {
                let mut ai = ai::Simple;
                let action = ai.action(id, self);
                self.current_location_mut().act(id, action);
            }
        }
        self.current_location_mut().post_turn();
        self.player_move = true;
    }
}
