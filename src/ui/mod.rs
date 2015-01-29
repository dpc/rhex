use std::old_io::Timer;
use std::time::duration::Duration;
use std::sync::{mpsc, Arc};
use std::collections::ring_buf::RingBuf;

use hex2d;

use actor;
use game;
use hex2dext::algo::bfs;

pub mod curses;

pub trait UiFrontend {
    fn input(&self) -> Option<Action>;
    fn draw(&mut self, astate : &actor::State, gstate : &game::State);
    fn event(&mut self, event : Event, gstate : &game::State);
}

pub enum Action {
    Exit,
    AutoExplore,
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
}

pub enum AutoExploreAction {
    Action(game::Action),
    Finish,
    Blocked,
}

impl<U : UiFrontend> Ui<U> {

    pub fn new(frontend : U) -> Ui<U> {
        Ui { frontend: frontend }
    }

    pub fn should_stop_autoexploring(&self, astate : &actor::State, _ : &game::State) -> bool {
        astate.discovered_areas.iter().count() > 0
    }

    pub fn autoexplore_action(&self, astate : &actor::State, gstate : &game::State) -> AutoExploreAction {

        let start = astate.pos;

        let mut bfs = bfs::Traverser::new(
            |pos| pos == start || gstate.tile_at(pos).map(|t| t.is_passable()) == Some(true),
            |pos| !astate.knows(pos),
            start
            );

        if let Some(dst) = bfs.find() {
            if let Some(neigh) = bfs.backtrace_last(dst) {

                let ndir = astate.pos.direction_to_cw(neigh).expect("bfs gave me trash");
                if ndir == astate.dir {
                    if gstate.occupied(neigh) {
                        AutoExploreAction::Blocked
                    } else {
                        AutoExploreAction::Action(game::Action::Move(hex2d::Angle::Forward))
                    }
                } else {
                    AutoExploreAction::Action(game::Action::Turn(ndir - astate.dir))
                }
            } else {
                AutoExploreAction::Finish
            }
        } else {
           AutoExploreAction::Finish
        }
    }

    pub fn run(&mut self,
               req : mpsc::Receiver<(Arc<actor::State>, Arc<game::State>)>,
               rep : mpsc::Sender<(Arc<actor::State>, game::Action)>
               ) {

        let mut autoexploring = None;
        let mut pending_req : Option<(Arc<actor::State>, Arc<game::State>)> = None;
        let mut pending_action = RingBuf::new();

        let mut timer = Timer::new().unwrap();

        loop {
            if let Some((astate, gstate)) = pending_req.clone() {
                if let Some(start_turn) = autoexploring {
                    if start_turn != gstate.turn && self.should_stop_autoexploring(&astate, &gstate) {
                        autoexploring = None;
                    } else {
                        match self.autoexplore_action(&astate, &gstate) {
                            AutoExploreAction::Blocked => {
                                autoexploring = None;
                            },
                            AutoExploreAction::Action(action) => {
                                rep.send((astate, action)).unwrap();
                                pending_req = None;
                                timer.sleep(Duration::milliseconds(50));
                            },
                            AutoExploreAction::Finish => {
                                self.frontend.event(Event::Log(LogEvent::AutoExploreDone), &gstate);
                                autoexploring = None;
                                self.frontend.draw(&astate, &gstate);
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
                            Action::AutoExplore => { autoexploring = Some(gstate.turn); },
                        }
                    }
                }
            } else {
                match req.try_recv() {
                    Ok(state) => {

                        {
                            let (ref astate, ref gstate) = state;
                            self.frontend.draw(&astate, &gstate);
                        }
                        pending_req = Some(state);
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
                    _ => {
                        autoexploring = None;
                        pending_action.push_back(action);
                    }
                }
            } else {
                timer.sleep(Duration::milliseconds(10));
            }


        }
    }
}

