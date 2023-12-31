use std::{env, process, sync::Arc};

use anyhow::{bail, Result};
use fixture_process::FixtureProcess;
use futures_util::{future::FusedFuture as _, pin_mut, select, FutureExt};
use server::Server;

use crate::{
    config::Config,
    utils::{ctrlc_2x, ResultExt},
};

mod cli;
mod config;
mod fixture_process;
mod logger;
mod server;
mod utils;

// TODO: tests
// TODO: docs
// TODO: Windows support (see cfg(unix))
// FIXME: killing the fixture doesn't work - kills the cargo test wrapper around it
//    - need to send SIGINT?
//    - or run fixture binary directly? -> No way to get the binary from cargo :|
//       - grep it out of the build output? -> ugly.
//       - -> can be done cleanly with --message-format=json-render-diagnostics
// FIXME: test args not passed (test name)

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
    // SIGINT handling:
    // The fixture process is set to use a new process group, ie. it doesn't receive SIGINTs.
    // The cargo test/-x process is created in the default (ours) group and gets SIGINT as usual,
    // it is then reaped by us.
    // We mostly ignore SIGINT, though when two quick SIGINTs (ie. "double click") come in,
    // we kill the fixture process - this provides a way to shut it down when it hangs.
    // For this purpose this ctrlc_2x future is created:
    let mut ctrlc_2x = ctrlc_2x()?;

    let config = Arc::new(config);

    // Build fixture program
    FixtureProcess::spawn_build(&config)?.await?;

    // Create a UDS server
    let server = Server::new(config.clone())?;

    // Run fixture program and accept its connection
    let fixture_ps = FixtureProcess::spawn_run(&config)?;
    pin_mut!(fixture_ps);
    let busy_logger = FixtureProcess::busy_logger("connected");

    let fixture_conn = loop {
        select! {
            res = server.accept_fixture().fuse() => break res?,
            res = fixture_ps => res?,
            _ = ctrlc_2x => fixture_ps.kill(),
        }
    };
    busy_logger.cancel().await;

    // Handle fixture connection and accept + handle test connections
    // the fixture connection handler runs cargo test
    let mut fixture_conn = smol::spawn(fixture_conn.run()).fuse();
    smol::spawn(server.accept_tests()).detach();

    // Wait for fixture connection and process to wrap up
    let test_res = loop {
        select! {
            res = fixture_ps => dbg!(res).log_error(),
            res = fixture_conn => break res,
            _ = ctrlc_2x => fixture_ps.kill(),
        }
    };

    if fixture_ps.is_terminated() {
        return test_res;
    };

    FixtureProcess::busy_logger("wrapped up").detach();
    loop {
        select! {
            res = fixture_ps => {
                res.log_error();
                return test_res;
            },
            _ = ctrlc_2x => fixture_ps.kill(),
        }
    }
}
