use crate::game;

/// Working with 256 color palette
mod color;

mod consts;

mod input;
mod map;

mod runner;
mod session;
pub use self::runner::Runner;
mod window;
use self::window::Window;

mod ui;

#[cfg(test)]
mod fixtures;

#[cfg(all(test, unix))]
#[path = "../../tests/support/parser.rs"]
mod parser;

pub enum LogEvent {
    AutoExploreDone,
    AutoExploreBlocked,
}

pub enum Event {
    Log(LogEvent),
}

pub struct LogEntry {
    turn: u64,
    text: String,
}

pub enum AutoMoveAction {
    Action(game::Action),
    Finish,  // Reached destination
    Blocked, // Blocked by something
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum GoToType {
    Stairs,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AutoMoveType {
    Explore,
    Walk,
    GoTo(GoToType),
}
