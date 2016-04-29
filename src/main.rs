#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate simplemap;
extern crate ncurses;
extern crate hex2d;
extern crate hex2d_dpcext as hex2dext;
extern crate rand;
extern crate num;
extern crate schedule_recv;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate fern;
extern crate fnv;

mod ai;
mod curses;
mod game;
mod generate;
mod util;
mod logging;

fn main() {
    logging::init();

    let mut ui = match curses::Ui::new() {
        Ok(res) => res,
        Err(err) => {
            match err {
                curses::Error::ColorCount => println!("rhex requires a terminal with 256 color support. Exiting."),
                _ => println!("An error occurred while initialising the UI. Exiting."),
            };
            std::process::exit(1);
        }
    };

    ui.run();
}
