use std::{env, process, sync::Arc};

use anyhow::{bail, Result};
use fixture_process::FixtureProcess;
use futures_util::{pin_mut, select, FutureExt};
use server::Server;
use utils::CtrlC;

use crate::config::Config;

mod cli;
mod config;
mod fixture_process;
mod logger;
mod server;
mod utils;

// TODO: tests
// TODO: docs
// TODO: ability to set -x from fixture (could be useful with project-defined Makefile/justfile etc.)
// TODO: clippy

const ENV_CARGO_FIXTURE: &str = "CARGO_FIXTURE";

fn main() -> Result<()> {
    if env::var_os(ENV_CARGO_FIXTURE).is_some() {
        bail!("Cannot run cargo fixture inside another cargo fixture");
    }
    env::set_var(ENV_CARGO_FIXTURE, "1");

    let cli = cli::parse()?;
    logger::init(cli.log_level);
    let config = Config::new(cli)?;

    let status = smol::block_on(serve(config))?;
    process::exit(status);
}

async fn serve(config: Config) -> Result<i32> {
    let ctrlc = CtrlC::new()?;
    let config = Arc::new(config);

    // Build fixture program
    FixtureProcess::spawn_build(&config, ctrlc.clone())?
        .join()
        .await?;

    // Run UDS server
    let mut server = smol::spawn(Server::new(config.clone(), ctrlc.clone())?.run()).fuse();

    // Run fixture program
    let fixture = FixtureProcess::spawn_run(&config, ctrlc)?;
    let fixture_join = fixture.join().fuse();

    // Wait for them to finish
    pin_mut!(fixture_join);
    select! {
        res = server => return res,
        res = fixture_join => res?,
    }
    server.await
}
