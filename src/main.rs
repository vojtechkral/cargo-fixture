use std::{env, process, sync::Arc};

use anyhow::{bail, Result};
use fixture_process::FixtureProcess;
use futures_util::{pin_mut, select, FutureExt};
use log::info;
use server::Server;
use utils::CtrlC;

use crate::config::Config;

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
        bail!("Cannot run cargo fixture inside another cargo fixture");
    }
    env::set_var(ENV_CARGO_FIXTURE, "1");

    let cli = cli::parse()?;
    logger::init(cli.log_level);
    let config = Config::new(cli)?;

    info!("setting up...");

    let status = smol::block_on(serve(config))?;
    process::exit(status);
}

async fn serve(config: Config) -> Result<i32> {
    let ctrlc = CtrlC::new()?;
    let config = Arc::new(config);
    let mut server = smol::spawn(Server::new(config.clone(), ctrlc.clone())?.run()).fuse();
    let fixture = FixtureProcess::spawn(&config, ctrlc)?;
    let fixture_join = fixture.join().fuse();
    pin_mut!(fixture_join);

    select! {
        res = server => return res,
        res = fixture_join => res?,
    }

    server.await
}
