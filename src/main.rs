extern crate chrono;
extern crate hex2d;
extern crate hex2d_dpcext as hex2dext;
extern crate num;
extern crate rand;
extern crate simplemap;
//#[macro_use]
extern crate fern;
extern crate fnv;
extern crate log;

mod ai;
mod game;
mod generate;
mod logging;
mod terminal;
mod util;

#[cfg(test)]
mod property_support;

fn main() {
    logging::init();

    if let Err(err) = run() {
        eprintln!("rhex: {err}");
        std::process::exit(1);
    }
}

fn run() -> std::io::Result<()> {
    let mut ui = terminal::Runner::new()?;
    ui.run()
}
