use super::{Location, Action};
use super::actor::{self, Actor};
use util;
use ai::{self, Ai};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum State {
    PlayerMove,
    AiMove,
    TurnDone,
}


pub struct Engine {
    turn: u64,
    location_cur: usize,
    locations: Vec<Location>,

    ids_to_move: Vec<actor::Id>,
    state : State,
}

impl Engine {
    pub fn new() -> Self {
        let location = Location::new(0);
        Engine {
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
        let pos = util::random_pos(0, 0);
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
        if actor_id == self.current_location().player_id() &&
           self.current_location().player().descended() {
            let mut player = self.current_location_mut().remove(actor_id).unwrap();
            self.location_cur += 1;
            self.locations.push(Location::new(self.location_cur as u32));
            player.pos = util::random_pos(0, 0);
            let _player = self.current_location_mut().spawn_player(player);
            self.end_turn();
        } else {
            self.state = State::AiMove;

            if self.ids_to_move.is_empty() {
                self.end_turn();
            }
        }
    }

    fn reload_actors_ids_to_move(&mut self) {
        let current_location = &self.locations[self.location_cur];
        let player_id = current_location.player_id();
        self.ids_to_move.extend(current_location
                                .actors_alive_ids()
                                .iter()
                                .filter(|&&id| id != player_id)
                                .cloned());
    }

    // player first move
    pub fn player_act(&mut self, action: Action) {
        assert!(self.state == State::PlayerMove);
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
            let action = ai.action(actor_id, self);
            self.current_location_mut().act(actor_id, action);
        } else {
            self.current_location_mut().skip_act(actor_id);
        }

        self.checks_after_act(actor_id);

        actor_id
    }

    fn end_turn(&mut self) {
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
