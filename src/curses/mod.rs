use ncurses as nc;

use std::cell::RefCell;
use game;

/// Working with 256 color palette
mod color;

mod consts;

mod map;

mod error;
pub use self::error::*;


mod ui;
pub use self::ui::*;

pub struct Window {
    pub window: nc::WINDOW,
}

impl Window {
    pub fn new(w: i32, h: i32, x: i32, y: i32) -> Window {
        Window { window: nc::subwin(nc::stdscr, h, w, y, x) }
    }

    pub fn clear(&self, calloc : &RefCell<color::Allocator>) {
        let color = color::BACKGROUND_BG;
        let cpair = nc::COLOR_PAIR(calloc.borrow_mut()
                                   .get(color::VISIBLE_FG, color));
        nc::wbkgd(self.window, cpair);
        nc::werase(self.window);
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        nc::delwin(self.window);
    }
}

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
