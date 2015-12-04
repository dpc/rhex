#![feature(core)]
#![feature(convert)]
#![feature(libc)]
#![feature(arc_unique)]
#![feature(core_str_ext)]
#![feature(slice_chars)]
#![feature(hashmap_hasher)]
#![allow(deprecated)]

extern crate simplemap;
extern crate ncurses;
extern crate hex2d;
extern crate hex2d_dpcext as hex2dext;
extern crate libc;
extern crate rand;
extern crate num;
extern crate schedule_recv;
extern crate time;
#[macro_use]
extern crate log;
extern crate fern;
extern crate core;
extern crate fnv;

use std::sync::mpsc;
use std::thread;

mod actor;
mod ai;
mod error;
mod game;
mod generate;
mod item;
mod ui;
mod util;
mod logging;

pub fn main() {

    logging::init();

    info!("Generating map...");
    let state = game::State::new();

    let mut controller = game::Controller::new(state);

    let (pl_req_tx, pl_req_rx) = mpsc::channel();
    let (pl_rep_tx, pl_rep_rx) = mpsc::channel();

    let (ai_req_tx, ai_req_rx) = mpsc::channel();
    let (ai_rep_tx, ai_rep_rx) = mpsc::channel();

    info!("Spawning game...");
    thread::spawn(move || {
        let _ = controller.run(
            pl_req_tx, pl_rep_rx,
            ai_req_tx, ai_rep_rx
            );
    });

    info!("Spawning AI...");
    thread::spawn(move|| {
        let _ = ai::run(ai_req_rx, ai_rep_tx);
    });

    info!("Spawning UI...");
    let mut ui = ui::curses::CursesUI::new();
    ui.display_intro();
    let mut ui = ui::Ui::new(ui);

    ui.run(pl_req_rx, pl_rep_tx);

    info!("UI done, exiting...");
}
