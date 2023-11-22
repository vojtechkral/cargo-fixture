use std::env;

use log::info;

use crate::{config::Config, fixture::FixtureProcess};

mod cli;
mod config;
mod fixture;
mod logger;
mod utils;

// TODO: error handling
// TODO: consider socket instead of std io pipe? - async support
// TODO: fixture data keep flag?

// cargo locate-project -> current Cargo.toml - nope, doesn't do -p => use metadata
// cargo metadata -> target dir

fn main() {
    // FIXME: check if already running under fixture
    env::set_var("CARGO_FIXTURE", "1");

    let cli = cli::parse();
    logger::init(cli.log_level);
    let config = Config::new(cli);

    info!("setting up...");
    let mut fixture = FixtureProcess::spawn(config).unwrap();
    fixture.serve();
    fixture.join();
}
