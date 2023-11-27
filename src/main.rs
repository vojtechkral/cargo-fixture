use std::{env, process};

use anyhow::{bail, Context as _, Result};
use async_ctrlc::CtrlC;
use log::info;

use crate::{config::Config, fixture::FixtureProcess, utils::ExitStatusExt};

mod cli;
mod config;
mod fixture;
mod logger;
mod utils;

// TODO: config through Cargo.toml meta??? (what config? feature?)
// TODO: fixture data keep flag? - nah

const ENV_CARGO_FIXTURE: &str = "CARGO_FIXTURE";

fn main() -> Result<()> {
    if env::var_os(ENV_CARGO_FIXTURE).is_some() {
        bail!("Cannot run cargo fixture inside another cargo fixture"); // TODO: test
    }
    env::set_var(ENV_CARGO_FIXTURE, "1");

    let cli = cli::parse();
    logger::init(cli.log_level);
    let config = Config::new(cli)?;

    info!("setting up...");

    let status = smol::block_on(async move {
        let ctrlc = CtrlC::new().context("Failed to create SIGINT handler")?;
        let mut fixture = FixtureProcess::spawn(config, ctrlc).await?;
        let status = fixture.serve().await?;
        fixture
            .join()
            .await?
            .as_result()
            .context("fixture teardown failure")?;

        Result::<_>::Ok(status)
    })?;

    process::exit(status);
}
