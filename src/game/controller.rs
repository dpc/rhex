use std::sync::{Arc, mpsc};
use rand;
use rand::Rng;
use std::old_io::Timer;
use std::time::Duration;

use game::{State, Action};
use error::Error;

pub type Request = (u32, Arc<State>);
pub type Reply = (u32, Action);


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

            for (&id, actor) in &actors {
                if actor.is_player() {
                        try!(pl_req.send(
                                (id, rc_state.clone())
                                ));
                } else {
                        try!(ai_req.send(
                                (id, rc_state.clone())
                                ));
                }
            }

            let mut actions = vec!();

            for (_, astate) in &actors {

                if !astate.can_perform_action() {
                    continue;
                }

                let (id, action) = if astate.is_player() {
                    try!(pl_rep.recv())
                } else {
                    try!(ai_rep.recv())
                };

                actions.push((id, action));
            }

            self.state.pre_tick();

            rand::thread_rng().shuffle(&mut actions);

            for (id, ref action) in actions {
                self.state.act(id, *action)
            }

            if self.state.descend {
                self.state = self.state.next_level();
            }

            timer.recv().unwrap();
        }
    }

}
