use std::io;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};

use super::session::Session;
use super::ui::Ui;
use crate::game::Engine;

/// Own terminal resources and drive the synchronous UI controller.
pub struct Runner {
    /// Restore terminal modes when the runner leaves scope.
    session: Session,
    /// Game and presentation state, independent of terminal ownership.
    ui: Ui,
}

impl Runner {
    /// Acquire the terminal before generating the initial game.
    pub fn new() -> io::Result<Self> {
        let session = Session::new()?;
        let ui = Ui::new(Engine::new(), Self::screen_size()?, Instant::now());
        Ok(Self { session, ui })
    }

    fn screen_size() -> io::Result<(i32, i32)> {
        let (width, height) = crossterm::terminal::size()?;
        Ok((height.into(), width.into()))
    }

    /// Preserve simulation, input, then rendering order until exit.
    pub fn run(&mut self) -> io::Result<()> {
        while !self.ui.exited() {
            self.ui.advance(Self::screen_size()?, Instant::now());
            // Crossterm's level-triggered Unix source needs a nonzero timeout.
            while event::poll(Duration::from_millis(1))? {
                let mut event = event::read()?;
                if matches!(event, Event::Resize(_, _)) {
                    // Resize notifications may be stale or coalesced.
                    let (height, width) = Self::screen_size()?;
                    event = Event::Resize(width as u16, height as u16);
                }
                self.ui.handle_event(event, Instant::now());
            }
            if self.ui.frame_due(Instant::now()) {
                self.session
                    .terminal
                    .borrow_mut()
                    .draw(|frame| self.ui.render_into(frame.buffer_mut()))?;
                self.ui.frame_drawn(Instant::now());
            } else if self.ui.should_idle() {
                thread::sleep(Duration::from_millis(10));
            }
        }
        Ok(())
    }
}
