// #![feature(log_syntax)]
// #![recursion_limit = "256"]
#![recursion_limit = "512"]

use std::{env, process};

use anyhow::{bail, Context as _, Ok, Result};
use async_ctrlc::CtrlC;
use log::info;

use crate::{config::Config, fixture::FixtureProcess, utils::ExitStatusExt};

mod cli;
mod cli_;
mod cli__;
// mod cli_nom;
mod cli_roll;
mod config;
mod fixture;
mod logger;
mod utils;

// FIXME: harness args require cargo fixture -- -- --arg
// TODO: rename data as tmpdata/tmp
// TODO: tests
// TODO: docs
// with-fixture fn args - env, data - nope
/* TODO: config through Cargo.toml meta???
    - feature name? - nejde, hardcoded v makru
    - fixture test name? - je to opravdu uzitecne?
*/

const ENV_CARGO_FIXTURE: &str = "CARGO_FIXTURE";

fn main() -> Result<()> {
    println!("{:#?}", cli_roll::FLAGS_);
    return Ok(());

    // let cli = cli_nom::parse();
    // dbg!(cli);
    // return Ok(());

    // let cli__ = cli__::parse();
    // dbg!(cli__);
    // return Ok(());

    let cli_ = cli_::cli_().run();
    dbg!(cli_);
    return Ok(());

    if env::var_os(ENV_CARGO_FIXTURE).is_some() {
        bail!("Cannot run cargo fixture inside another cargo fixture"); // TODO: test
    }
    env::set_var(ENV_CARGO_FIXTURE, "1");

    let cli = cli::parse();
    dbg!(cli);
    return Ok(());
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
