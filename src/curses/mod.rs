use game;

/// Working with 256 color palette
mod color;

mod consts;

mod error;
pub use self::error::*;


mod ui;
pub use self::ui::*;

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
    Finish, // Reached destination
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
