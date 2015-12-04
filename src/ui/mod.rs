use std::sync::{mpsc};
use std::collections::VecDeque;
use std::thread;
use time;
use game::controller::{Request, Reply};
use hex2d;

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
    AutoExplore,
    Redraw,
    Game(game::Action),
}

pub enum LogEvent {
    AutoExploreDone,
}

pub enum Event {
    Log(LogEvent)
}

/// Generic UI logic
pub struct Ui<U : UiFrontend> {
    frontend : U,
    autoexploring : Option<u64>,
    last_redraw_ns : u64,
}

pub enum AutoExploreAction {
    Action(game::Action),
    Finish,
    Blocked,
}

impl<U : UiFrontend> Ui<U> {

    pub fn new(frontend : U) -> Ui<U> {
        Ui {
            frontend: frontend,
            autoexploring: None,
            last_redraw_ns: 0,
        }
    }

    pub fn should_stop_autoexploring(
        &self, astate : &actor::State, gstate : &game::State) -> bool {

        !astate.was_attacked_by.is_empty() ||
        astate.discovered_areas.iter().any(|_| true ) ||
        astate.visible.iter().any(|&coord|
                                  gstate.at(coord)
                                  .actor_map_or(false, |a| a.race == actor::Race::Grue)
                                 ) ||
        astate.discovered.iter().any(|&coord|
                                     gstate.at(coord)
                                     .item_map_or(false, |_| true)
                                    ) ||
        astate.heared.iter()
        .filter(|&(c, t)| *c != astate.pos.coord && *t != Noise::Creature(Race::Pony))
        .any(|(c, _)| !astate.sees(*c)) ||
        astate.discovered_stairs(gstate)
    }

    pub fn autoexplore_action(
        &self, astate : &actor::State, gstate : &game::State
        ) -> AutoExploreAction {

        let start = astate.pos.coord;

        let mut bfs = bfs::Traverser::new(
            |c| c == start || gstate.at(c).tile().is_passable(),
            |c| !astate.knows(c),
            start
            );

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {

                let ndir = astate.pos.coord.direction_to_cw(neigh).expect("bfs gave me trash");
                if ndir == astate.pos.dir {
                    if gstate.at(neigh).is_occupied() {
                        AutoExploreAction::Blocked
                    } else {
                        AutoExploreAction::Action(game::Action::Move(hex2d::Angle::Forward))
                    }
                } else {
                    AutoExploreAction::Action(game::Action::Turn(ndir - astate.pos.dir))
                }
            } else {
                AutoExploreAction::Finish
            }
        } else {
           AutoExploreAction::Finish
        }
    }

    pub fn redraw(&mut self, req : &Option<Request>) {
        if let &Some((id, ref gstate)) = req {
            let now = time::precise_time_ns();

            if self.autoexploring.is_some() && self.last_redraw_ns + 50 * 1000 * 1000 > now {
                return
            }

            self.last_redraw_ns = now;

            let astate = &gstate.actors[&id];
            self.frontend.draw(&astate, &gstate);
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
                let astate = &gstate.actors[&id];
                if let Some(start_turn) = self.autoexploring {
                    if start_turn != gstate.turn && self.should_stop_autoexploring(&astate, &gstate) {
                        self.autoexploring = None;
                        self.redraw(&pending_req);
                    } else {
                        match self.autoexplore_action(&astate, &gstate) {
                            AutoExploreAction::Blocked => {
                                self.autoexploring = None;
                                self.redraw(&pending_req);
                            },
                            AutoExploreAction::Action(action) => {
                                rep.send((id, action)).unwrap();
                                pending_req = None;
                                thread::sleep_ms(10);
                            },
                            AutoExploreAction::Finish => {
                                self.frontend.event(Event::Log(LogEvent::AutoExploreDone), &gstate);
                                self.autoexploring = None;
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
                            Action::AutoExplore => { self.autoexploring = Some(gstate.turn); },
                        }
                    }
                }
            } else {
                match req.try_recv() {
                    Ok(req) => {
                        let skip = {
                            let (id, ref gstate) = req;
                            let astate = &gstate.actors[&id];
                            self.frontend.update(&astate, &gstate);
                            !astate.can_perform_action()
                        };

                        pending_req = Some(req);
                        self.redraw(&pending_req);

                        if skip {
                            // no need to respond
                            pending_req = None;
                        }
                    },
                    Err(err) => match err {
                        mpsc::TryRecvError::Empty => {},
                        _ => panic!(),
                    }
                }
            }

            if let Some(action) = self.frontend.input(
                pending_req.as_ref().map(|&(id, ref gstate)| &gstate.actors[&id])
                ) {
                match action {
                    Action::Exit => {
                        debug!("Received Action::Exit");
                        return
                    },
                    Action::AutoExplore => {
                        if self.autoexploring.is_none() {
                            pending_action.push_back(action);
                        }
                    },
                    Action::Redraw => {
                        self.redraw(&pending_req);
                    },
                    _ => {
                        if self.autoexploring.is_some() {
                            self.autoexploring = None;
                            self.redraw(&pending_req);
                        }
                        pending_action.push_back(action);
                    }
                }
            } else {
                thread::sleep_ms(10);
            }
        }
    }
}

