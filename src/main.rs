use std::env;

use anyhow::{Result, Context as _};
use async_ctrlc::CtrlC;
use log::info;

use crate::{config::Config, fixture::FixtureProcess};

mod cli;
mod config;
mod fixture;
mod logger;
mod utils;

// TODO: error handling
// TODO: fixture data keep flag?
// TODO: ctrl-c

// cargo locate-project -> current Cargo.toml - nope, doesn't do -p => use metadata
// cargo metadata -> target dir

fn main() {
    // FIXME: check if already running under fixture
    env::set_var("CARGO_FIXTURE", "1");

    let cli = cli::parse();
    logger::init(cli.log_level);
    let config = Config::new(cli);

    info!("setting up...");

    // FIXME: set smol max blocking threads to reasonable value https://docs.rs/blocking/latest/blocking/index.html
    let res = smol::block_on(async move {
        let ctrlc = CtrlC::new().context("Failed to create SIGINT handler")?;
        let mut fixture = FixtureProcess::spawn(config, ctrlc).await?;
        fixture.serve().await?;
        fixture.join().await;

        Result::<()>::Ok(())
    });

    res.unwrap(); // FIXME:
}
