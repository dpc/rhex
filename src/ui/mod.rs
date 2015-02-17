use std::old_io::Timer;
use std::time::duration::Duration;
use std::sync::{mpsc, Arc};
use std::collections::ring_buf::RingBuf;
use time;

use hex2d;

use actor;
use game;
use hex2dext::algo::bfs;

pub mod curses;

pub trait UiFrontend {
    fn update(&mut self, astate : &actor::State, gstate : &game::State);
    fn draw(&mut self, astate : &actor::State, gstate : &game::State);
    fn input(&mut self) -> Option<Action>;
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

    pub fn should_stop_autoexploring(&self, astate : &actor::State, _ : &game::State) -> bool {
        astate.discovered_areas.iter().count() > 0
    }

    pub fn autoexplore_action(&self, astate : &actor::State, gstate : &game::State) -> AutoExploreAction {

        let start = astate.pos.coord;

        let mut bfs = bfs::Traverser::new(
            |c| c == start || gstate.tile_at(c).map(|t| t.is_passable()) == Some(true),
            |c| !astate.knows(c),
            start
            );

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {

                let ndir = astate.pos.coord.direction_to_cw(neigh).expect("bfs gave me trash");
                if ndir == astate.pos.dir {
                    if gstate.is_occupied(neigh) {
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

    pub fn redraw(&mut self, req : &Option<game::controller::Request>) {
        let now = time::precise_time_ns();

        if self.autoexploring.is_some() && self.last_redraw_ns + 50 * 1000 * 1000 > now {
            return
        }

        self.last_redraw_ns = now;

        if let Some((ref astate, ref gstate)) = *req {
            self.frontend.draw(&astate, &gstate);
        }
    }

    pub fn run(&mut self,
               req : mpsc::Receiver<(Arc<actor::State>, Arc<game::State>)>,
               rep : mpsc::Sender<(Arc<actor::State>, game::Action)>
               ) {

        let mut pending_req : Option<(Arc<actor::State>, Arc<game::State>)> = None;
        let mut pending_action = RingBuf::new();

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
                    Ok(state) => {
                        {
                            let (ref astate, ref gstate) = state;
                            self.frontend.update(&astate, &gstate);
                        }
                        pending_req = Some(state);
                        self.redraw(&pending_req);
                    },
                    Err(err) => match err {
                        mpsc::TryRecvError::Empty => {},
                        _ => panic!(),
                    }
                }
            }

            if let Some(action) = self.frontend.input() {
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

