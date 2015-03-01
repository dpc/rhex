use std::old_io::Timer;
use std::time::duration::Duration;
use std::sync::{mpsc};
use std::collections::VecDeque;
use time;
use game::controller::{Request, Reply};
use hex2d;

use actor;
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

    pub fn should_stop_autoexploring(&self, astate : &actor::State, gstate : &game::State) -> bool {
        astate.discovered_areas.iter().any(|_| true ) ||
            astate.visible.iter().any(|&coord|
                                      gstate.at(coord).actor_map_or(false, |a| a.behavior == actor::Behavior::Grue)
                                      ) ||
            astate.discovered.iter().any(|&coord|
                                      gstate.at(coord).item_map_or(false, |_| true)
                                      ) ||
            astate.heared.iter()
                .filter(|&c| *c != astate.pos.coord)
                .any(|&c| !astate.sees(c)) ||
            astate.discovered_stairs(gstate)
    }

    pub fn autoexplore_action(&self, astate : &actor::State, gstate : &game::State) -> AutoExploreAction {

        let start = astate.pos.coord;

        let mut bfs = bfs::Traverser::new(
            |c| c == start || gstate.at(c).tile_map_or(false, |t| t.is_passable()),
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
        if let &Some((ref astate, ref gstate)) = req {
            let now = time::precise_time_ns();

            if self.autoexploring.is_some() && self.last_redraw_ns + 50 * 1000 * 1000 > now {
                return
            }

            self.last_redraw_ns = now;

            self.frontend.draw(&astate, &gstate);
        }
    }

    pub fn run(&mut self,
               req : mpsc::Receiver<Request>,
               rep : mpsc::Sender<Reply>
               ) {

        let mut pending_req : Option<Request> = None;
        let mut pending_action = VecDeque::new();

        let mut timer = Timer::new().unwrap();

        loop {
            if let Some((astate, gstate)) = pending_req.clone() {
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
                                rep.send((astate, action)).unwrap();
                                pending_req = None;
                                timer.sleep(Duration::milliseconds(10));
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
                                rep.send((astate, action)).unwrap();
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
                            let (ref astate, ref gstate) = req;
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

            if let Some(action) = self.frontend.input(pending_req.as_ref().map(|&(ref astate, ref _gstate)| &**astate)) {
                match action {
                    Action::Exit => return,
                    Action::AutoExplore => {
                        pending_action.push_back(action);
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
                timer.sleep(Duration::milliseconds(10));
            }
        }
    }
}

