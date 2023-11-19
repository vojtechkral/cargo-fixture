use log::info;

use crate::fixture::FixtureProcess;

mod cli;
mod fixture;
mod logger;
mod utils;

// TODO: error handling
// TODO: consider socket instead of std io pipe? - async support
// TODO: fixture data keep flag?

fn main() {
    let cli = cli::parse();
    logger::init(cli.verbosity());

    info!("setting up...");
    let mut fixture = FixtureProcess::spawn(cli).unwrap();
    fixture.serve();
    fixture.join();
}
