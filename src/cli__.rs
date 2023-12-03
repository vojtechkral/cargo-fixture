use std::{
    env,
    ffi::{OsStr, OsString},
    iter,
};

use clap::{value_parser, Arg, ArgAction, ArgMatches, FromArgMatches, Parser};
use os_str_bytes::RawOsStr;

use crate::logger::LogLevel;

pub fn parse() -> Cli {
    // FIXME: explain
    let mut args = env::args_os().skip(1).peekable();
    let arg0 = OsString::from("cargo fixture");
    if args.peek().map(|arg| arg.as_os_str()) == Some(OsStr::new("fixture")) {
        args.next().unwrap();
        // TODO: test this
    }
    let args = iter::once(arg0).chain(args);
    match Commands::parse_from(args) {
        Commands::Fixture(fixture) => fixture,
    }
}

#[derive(Parser)]
#[command(multicall = true, disable_version_flag = true)]
enum Commands {
    #[command(name = "cargo fixture", version)]
    Fixture(Cli),
}

#[derive(Parser, Debug)]
pub struct Cli {
    /// Pass a flag/argument to the fixture binary; use multiple times to pass several arguments
    #[arg(short = 'A', value_name = "FLAG|ARG", allow_hyphen_values = true)]
    pub fixture_args: Vec<String>,

    /// Set stderr logging level
    #[arg(short = 'L', value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    // TODO: keep fixture data flag?
    /// Instead of running cargo test [args...] run the specified command and pass it all remaining arguments
    #[arg(short = 'x', allow_hyphen_values = true, num_args = 1.., value_name = "ARGS")]
    pub exec: Vec<OsString>,

    /// Print version
    #[arg(long, action = ArgAction::Version)]
    version: (),

    // /// cargo args
    // // #[arg(allow_hyphen_values = true)]
    // pub cargo_args: Vec<OsString>,

    // /// harness args
    // #[arg(last = true)]
    // pub harness_args: Vec<OsString>,
    /// rest
    #[arg(allow_hyphen_values = true, num_args = 1.., value_name = "ARGS")]
    pub rest: Vec<OsString>,
}
