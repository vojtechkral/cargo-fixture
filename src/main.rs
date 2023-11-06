use std::{env, mem};

use clap::Parser;
use fixture::CmdSpec;
use log::info;

use crate::fixture::FixtureProcess;

mod fixture;
mod logger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// stderr verbosity
    #[arg(short, action = clap::ArgAction::Count, default_value_t = 0)]
    verbosity: u8,

    /// no stderr logging (overrides -v)
    #[arg(short, default_value_t = false)]
    quiet: bool,

    // /// FIXME:
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    test_cmd: Vec<String>,
}

impl Cli {
    fn take_test_cmd(&mut self, cargo: &str) -> CmdSpec {
        if self.test_cmd.is_empty() {
            CmdSpec::new(cargo.to_string(), vec!["test".to_string()])
        } else {
        CmdSpec::new(cargo.to_string(), mem::take(&mut self.test_cmd))}
    }
}

impl Cli {
    fn verbosity(&self) -> u32 {
        if self.quiet {
            0
        } else {
            self.verbosity as u32 + 1
        }
    }
}

fn main() {
    // cargo always passes the name of the command, ie. `fixture`
    // as the first argument, so we need to filter that out if that's the case.
    let args = env::args_os().enumerate().filter_map(|(i, arg)| {
        if i == 1 && arg == "fixture" {
            None
        } else {
            Some(arg)
        }
    });
    let mut cli = Cli::parse_from(args);
    logger::init(cli.verbosity());

    let cargo = env::var("CARGO").unwrap_or_else(|_| {
        env::set_var("CARGO", "cargo");
        "cargo".to_string()
    });

    info!("setting up...");
    // FIXME: Customizable? ie. workspace package etc.
    //        use cargo metadata? https://doc.rust-lang.org/cargo/reference/manifest.html#the-metadata-table
    let fixture_cmd = CmdSpec::new(cargo.clone(), ["test", "--test", "fixture"].into_iter().map(|s| s.to_string()).collect());
    let test_cmd = cli.take_test_cmd(&cargo);
    let mut fixture = FixtureProcess::spawn(fixture_cmd, test_cmd).unwrap();
    fixture.serve();
    fixture.join();
}
