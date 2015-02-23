use std::sync::{Arc, mpsc};
use rand;
use rand::Rng;
use std::old_io::Timer;
use std::time::Duration;

use actor;
use game::{State, Action, Stage};
use error::Error;

pub type Request = (Arc<actor::State>, Arc<State>);
pub type Reply = (Arc<actor::State>, Action);


/// Controller between actors behavior engines and `game::State`
///
/// Meant to be running in it's own thread
pub struct Controller {
    state : State,
}

impl Controller {
    pub fn new(state : State) -> Controller {
        Controller {
            state: state,
        }
    }

    pub fn run(&mut self,
               pl_req : mpsc::Sender<Request>,
               pl_rep : mpsc::Receiver<Reply>,
               ai_req : mpsc::Sender<Request>,
               ai_rep : mpsc::Receiver<Reply>,
               ) -> Result<(), Error<Request>>
    {
        let mut timer = Timer::new().unwrap();
        let timer = timer.periodic(Duration::milliseconds(100));


        loop {
            self.state.post_tick();

            let rc_state = Arc::new(self.state.clone());
            let actors = self.state.actors.clone();

            for (&acoord, actor) in &actors {
                match actor.behavior {
                    actor::Behavior::Player => {
                        try!(pl_req.send(
                                (rc_state.actors[acoord].clone(), rc_state.clone())
                                ));
                    },
                    actor::Behavior::Grue|actor::Behavior::Pony => {
                        try!(ai_req.send(
                                (rc_state.actors[acoord].clone(), rc_state.clone())
                                ));
                    },
                }
            }

            for astate in &rc_state.actors_dead {
                match astate.behavior {
                    actor::Behavior::Player => {
                        try!(pl_req.send(
                                (astate.clone(), rc_state.clone())
                                ));
                    },
                    _ => {},
                };
            }

            let mut actions = vec!();

            for (_, astate) in &actors {
                let (acoord, action) = match astate.behavior {
                    actor::Behavior::Player => {
                        try!(pl_rep.recv())
                    },
                    actor::Behavior::Grue|actor::Behavior::Pony => {
                        try!(ai_rep.recv())
                    },
                };

                actions.push((acoord, action));
            }

            self.state.pre_tick();

            rand::thread_rng().shuffle(&mut actions);

            for &(ref astate, ref action) in &actions {
                self.state.act(Stage::ST1, astate.pos.coord, *action)
            }

            for &(ref astate, ref action) in &actions {
                self.state.act(Stage::ST2, astate.pos.coord, *action)
            }
            timer.recv().unwrap();
        }
    }

}
