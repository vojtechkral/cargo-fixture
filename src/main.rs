use std::{env, process, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use fixture_process::FixtureProcess;
use futures_util::{pin_mut, select, FutureExt};
use server::Server;
use utils::CtrlC;

use crate::{config::Config, utils::timeout};

mod cli;
mod config;
mod fixture_process;
mod logger;
mod server;
mod utils;

// TODO: tests
// TODO: docs
// FIXME: cargo.toml error -> opaque cargo metadata error

const FIXTURE_FEATURE: &str = "_fixture"; // kept in sync with macro
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
    pin_mut!(fixture_join);

    // Wait for them to finish
    // We observe both futures, but ultimately the server's result
    // is returned, though fixture errors are printed as well.
    select! {
        res = fixture_join => {
            if let Err(err) = res {
                eprintln!("Error: {err:?}");
            }
            server.await
        },
        res = server => {
            // Give the fixture process a bit of time to exit as well
            if let Some(Err(err)) = timeout(fixture_join, Duration::from_millis(250)).await {
                eprintln!("Error: {err:?}");
            }
            res
        },
    }
}
