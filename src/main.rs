use std::{env, process};

use anyhow::{Context as _, Result};
use async_ctrlc::CtrlC;
use log::info;

use crate::{config::Config, fixture::FixtureProcess};

mod cli;
mod config;
mod fixture;
mod logger;
mod utils;

// TODO: error handling
// TODO: config through Cargo.toml meta???
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

    let res = smol::block_on(async move {
        let ctrlc = CtrlC::new().context("Failed to create SIGINT handler")?;
        let mut fixture = FixtureProcess::spawn(config, ctrlc).await?;
        let status = fixture.serve().await?;
        fixture.join().await;

        Result::<_>::Ok(status)
    });

    let status = res.unwrap(); // FIXME:
    process::exit(status)
}
