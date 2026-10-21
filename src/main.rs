extern crate chrono;
extern crate hex2d;
extern crate hex2d_dpcext as hex2dext;
extern crate ncurses;
extern crate num;
extern crate rand;
extern crate simplemap;
//#[macro_use]
extern crate fern;
extern crate fnv;
extern crate log;

mod ai;
mod curses;
mod game;
mod generate;
mod logging;
mod util;

fn main() {
    logging::init();

    let mut ui = match curses::Ui::new() {
        Ok(res) => res,
        Err(err) => {
            match err {
                curses::Error::ColorCount => {
                    println!("rhex requires a terminal with 256 color support. Exiting.")
                }
                _ => println!("An error occurred while initialising the UI. Exiting."),
            };
            std::process::exit(1);
        }
    };

    ui.run();
}
