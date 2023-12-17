use std::{env, ffi::OsString, process};

use anyhow::Result;

use crate::logger::LogLevel;

mod flags;
use flags::{def_flags, FlagDef};
mod parser;
use parser::Parser;

#[derive(Default, Debug)]
pub struct Cli {
    pub log_level: LogLevel,
    pub fixture_args: Vec<OsString>,
    pub exec: Vec<OsString>,
    /// FIXME: docs
    pub cargo_common_all: Vec<OsString>,
    /// FIXME: docs
    pub cargo_common_test: Vec<OsString>,
    pub cargo_test_args: Vec<OsString>,
    pub harness_args: Vec<OsString>,
}

def_flags!(
    // cargo fixture args
    -L                    parse_value(log_level) "TODO:",
    -A                    append_value_raw(fixture_args) "TODO:",
    -x --exec [Args...]   take_remaining(exec) "Instead of running cargo test [args...], run the specified command and pass it all remaining arguments",
    -h --help             help "TODO:",
    --version             version "TODO:",

    // Common cargo args
    -q --quiet                forward(cargo_common_all),
    -v --verbose              forward(cargo_common_all),
    -Z [FLAG]                 forward_value(cargo_common_all),
    --color [WHEN]            forward_value(cargo_common_all),
    --config [KEY=VALUE]      forward_value(cargo_common_all),
    -F --features [FEATURES]  forward_value(cargo_common_all),
    --all-features            forward(cargo_common_all),
    --no-default-features     forward(cargo_common_all),
    --manifest-path [PATH]    forward_value(cargo_common_all),
    --frozen                  forward(cargo_common_all),
    --locked                  forward(cargo_common_all),
    --offline                 forward(cargo_common_all),

    // Common cargo test args
    --ignore-rust-version    forward(cargo_common_test),
    --future-incompat-report forward(cargo_common_test),
    -p --package [SPEC]      forward_value(cargo_common_test), // TODO: We might need to extract this one too (?) - to get Cargo.toml meta config
    -j --jobs [N]            forward_value(cargo_common_test),
    -r --release             forward(cargo_common_test),
    --profile [NAME]         forward_value(cargo_common_test),
    --target [TRIPLE]        forward_value(cargo_common_test),
    --target-dir [DIR]       forward_value(cargo_common_test),
    --unit-graph             forward(cargo_common_test),
    --timings [FORMATS]      forward_value(cargo_common_test),
);

pub fn parse() -> Result<Cli> {
    Parser::new(FLAGS, env::args_os())?.parse().map_err(|err| {
        eprintln!("{err}");
        process::exit(err.exit_code());
    })
}
