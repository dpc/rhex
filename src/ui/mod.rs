use std::sync::{mpsc};
use std::collections::VecDeque;
use std::thread;
use time;
use game::controller::{Request, Reply};
use hex2d;

use hex2d::Angle;

use actor::{self, Noise, Race};
use game;
use hex2dext::algo::bfs;

pub mod curses;

pub trait UiFrontend {
    fn update(&mut self, astate : &actor::State, gstate : &game::State);
    fn draw(&mut self, astate : &actor::State, gstate : &game::State);
    fn input(&mut self, astate : Option<&actor::State>) -> Option<Action>;
    fn event(&mut self, event : Event, gstate : &game::State);
}

pub enum Action {
    Exit,
    AutoMove(AutoMoveType),
    Redraw,
    Game(game::Action),
}

/// Generic UI logic
pub struct Ui<U : UiFrontend> {
    frontend : U,
    automoving : Option<AutoMoveType>,
    automove_turn : u64,
    last_redraw_ns : u64,
}

impl<U : UiFrontend> Ui<U> {

    pub fn new(frontend : U) -> Ui<U> {
        Ui {
            frontend: frontend,
            automoving : None,
            automove_turn: 0,
            last_redraw_ns: 0,
        }
    }

    pub fn run(&mut self,
               req : mpsc::Receiver<Request>,
               rep : mpsc::Sender<Reply>
               ) {

        let mut pending_req : Option<Request> = None;
        let mut pending_action = VecDeque::new();

        loop {
            if let Some((id, gstate)) = pending_req.clone() {
                let astate = &gstate.actors_byid[&id];
                if let Some(movetype) = self.automoving {
                    let start_turn = self.automove_turn;
                    if start_turn != gstate.turn && self.should_stop_automoving(&astate, &gstate) {
                        self.automoving = None;
                        self.redraw(&pending_req);
                    } else {
                        match self.automove_action(&astate, &gstate, movetype) {
                            AutoMoveAction::Blocked => {
                                self.automoving = None;
                                self.redraw(&pending_req);
                            },
                            AutoMoveAction::Action(action) => {
                                rep.send((id, action)).unwrap();
                                pending_req = None;
                                thread::sleep_ms(10);
                            },
                            AutoMoveAction::Finish => {
                                if movetype == AutoMoveType::Explore {
                                    self.frontend.event(
                                        Event::Log(LogEvent::AutoExploreDone), &gstate
                                        );
                                }
                                self.automoving = None;
                                self.redraw(&pending_req);
                            }
                        }
                    }
                } else {
                    if let Some(action) = pending_action.pop_front() {
                        match action {
                            Action::Exit => return, // Shouldn't really be there, but whatever...
                            Action::Game(action) => {
                                rep.send((id, action)).unwrap();
                                pending_req = None;
                            },
                            Action::Redraw => { },
                            Action::AutoMove(movetype) => {
                                self.automoving = Some(movetype);
                                self.automove_turn = gstate.turn;
                            },
                        }
                    }
                }
            } else {
                match req.try_recv() {
                    Ok(req) => {
                        let skip = {
                            let (id, ref gstate) = req;
                            let astate = &gstate.actors_byid[&id];
                            self.frontend.update(&astate, &gstate);
                            !astate.can_perform_action()
                        };

                        pending_req = Some(req);
                        self.redraw(&pending_req);

                        if skip {
                            // no need to respond
                            pending_req = None;
                        } else {

                        }
                    },
                    Err(err) => match err {
                        mpsc::TryRecvError::Empty => {},
                        _ => panic!(),
                    }
                }
            }

            if let Some(action) = self.frontend.input(
                pending_req.as_ref().map(|&(id, ref gstate)| &gstate.actors_byid[&id])
                ) {
                match action {
                    Action::Exit => {
                        debug!("Received Action::Exit");
                        return
                    },
                    Action::Redraw => {
                        self.redraw(&pending_req);
                    },
                    _ => {
                        if self.automoving.is_some() {
                            self.automoving = None;
                            self.redraw(&pending_req);
                        } else {
                            pending_action.push_back(action);
                        }

                    }
                }
            } else {
                thread::sleep_ms(10);
            }
        }
    }
}

