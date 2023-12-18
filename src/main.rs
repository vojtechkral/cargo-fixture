use std::env;

use anyhow::{bail, Context as _, Ok, Result};
use async_ctrlc::CtrlC;
use fixture_process::FixtureProcess;
use log::info;
use server::Server;

use crate::{config::Config, utils::ExitStatusExt};

mod cli;
mod config;
mod fixture;
mod fixture_process;
mod logger;
mod server;
mod utils;

// TODO: rename data as tmpdata/tmp (??) -> nah, move to in-memory stuff with the new client
// TODO: tests
// TODO: docs
// with-fixture fn args - env, data - nope
/* TODO: config through Cargo.toml meta???
    - feature name? - nejde, hardcoded v makru
    - fixture test name? - je to opravdu uzitecne?
*/

const ENV_CARGO_FIXTURE: &str = "CARGO_FIXTURE";

fn main() -> Result<()> {
    if env::var_os(ENV_CARGO_FIXTURE).is_some() {
        bail!("Cannot run cargo fixture inside another cargo fixture"); // TODO: test
    }
    env::set_var(ENV_CARGO_FIXTURE, "1");

    let cli = cli::parse()?;
    logger::init(cli.log_level);
    let config = Config::new(cli)?;

    info!("setting up...");

    let status = smol::block_on(serve(config))?;

    // process::exit(status);
    Ok(())
}

async fn serve(config: Config) -> Result<()> {
    let ctrlc = CtrlC::new().context("Failed to create a SIGINT handler")?;
    let fixure_cmd = config.fixture_cmd();

    // let mut fixture = FixtureProcess::spawn(config, ctrlc).await?;
    // let status = fixture.serve().await?;
    // fixture
    //     .join()
    //     .await?
    //     .as_result()
    //     .context("fixture teardown failure")?;

    let server = smol::spawn(Server::new(config)?.run());
    FixtureProcess::spawn(fixure_cmd, ctrlc)?
        .await?
        .as_result()
        .context("Fixture teardown failure")?;

    server.await
}
