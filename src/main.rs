use std::env;

use clap::Parser;
use log::info;

use crate::{fixture::FixtureProcess, tests_runner::run_tests};

mod fixture;
mod logger;
mod tests_runner;

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
    fn test_cmd(&self) -> &[String] {
        // cargo always passes the name of the command, ie. `fixture` in this case
        // as the first argument, so we need to exclude it here.
        &self.test_cmd[1..]
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
    let cli = Cli::parse();
    logger::init(cli.verbosity());

    let cargo = env::var("CARGO").unwrap_or_else(|_| {
        env::set_var("CARGO", "cargo");
        "cargo".to_string()
    });

    info!("setting up...");
    let mut fixture = FixtureProcess::spawn(&cargo).unwrap();
    fixture.serve();
    fixture.join();
}
