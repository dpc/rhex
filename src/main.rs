#![feature(core)]
#![feature(std_misc)]
#![feature(io)]
#![feature(libc)]
#![feature(alloc)]
#![feature(env)]
#![feature(os)]

extern crate ncurses;
extern crate hex2d;
extern crate "hex2d-dpcext" as hex2dext;
extern crate libc;
extern crate rand;

use std::sync::mpsc;
use std::thread;

use hex2d::Coordinate;

mod ai;
mod ui;
mod game;
mod actor;
mod generate;

pub fn main() {

    let state = game::State::new();
    let state = state.spawn_player();
    let state = state.spawn_monster();
    let state = state.spawn_pony(Coordinate::new(-1, 0));
    let mut controller = game::Controller::new(state);

    let ui = ui::curses::CursesUI::new();
    let mut ui = ui::Ui::new(ui);

    let (pl_req_tx, pl_req_rx) = mpsc::channel();
    let (pl_rep_tx, pl_rep_rx) = mpsc::channel();

    let (ai_req_tx, ai_req_rx) = mpsc::channel();
    let (ai_rep_tx, ai_rep_rx) = mpsc::channel();

    thread::Thread::spawn(move|| {
        let _ = controller.run(
            pl_req_tx, pl_rep_rx,
            ai_req_tx, ai_rep_rx
            );
    });

    thread::Thread::spawn(move|| {
        ai::run(ai_req_rx, ai_rep_tx);
    });


    ui.run(pl_req_rx, pl_rep_tx);
}
