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
    state : Arc<State>,
}


impl Controller {
    pub fn new(state : State) -> Controller {
        Controller {
            state: Arc::new(state),
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

            self.state = Arc::new(self.state.tick());
            let actors = self.state.actors.clone();

            for (_, actor) in &*actors {
                match actor.behavior {
                    actor::Behavior::Player => {
                        try!(pl_req.send((actor.clone(), self.state.clone())));
                    },
                    actor::Behavior::Grue|actor::Behavior::Pony => {
                        try!(ai_req.send((actor.clone(), self.state.clone())));
                    },
                }
            }

            for astate in &*self.state.actors_dead {
                match astate.behavior {
                    actor::Behavior::Player => {
                        try!(pl_req.send((astate.clone(), self.state.clone())));
                    },
                    _ => {},
                };
            }

            let mut actions = vec!();

            for (_, astate) in &*actors {
                let (astate, action) = match astate.behavior {
                    actor::Behavior::Player => {
                        try!(pl_rep.recv())
                    },
                    actor::Behavior::Grue|actor::Behavior::Pony => {
                        try!(ai_rep.recv())
                    },
                };

                actions.push((astate.pos.coord, action));

            }

            rand::thread_rng().shuffle(&mut actions);

            for &(acoord, action) in &actions {
                self.state = Arc::new(self.state.act(Stage::ST1, acoord, action));
            }

            for &(acoord, action) in &actions {
                self.state = Arc::new(self.state.act(Stage::ST2, acoord, action));
            }
            timer.recv().unwrap();
        }
    }

}
