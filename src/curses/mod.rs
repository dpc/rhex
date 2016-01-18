use game;

mod color;

mod consts;

mod error;
pub use self::error::*;


mod ui;
pub use self::ui::*;

pub enum LogEvent {
    AutoExploreDone,
}

pub enum Event {
    Log(LogEvent)
}

pub struct LogEntry {
    turn : u64,
    text : String,
}

pub enum AutoMoveAction {
    Action(game::Action),
    Finish,
    Blocked,
}


#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AutoMoveType {
    Explore,
    Walk,
}


