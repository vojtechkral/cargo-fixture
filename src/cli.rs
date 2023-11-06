use std::{env, ffi::OsString, process};

use anyhow::Result;

use crate::logger::LogLevel;

mod flags;
use flags::def_flags;
mod parser;
use parser::Parser;

def_flags!(
    FLAGS:

    --fixture [name]      parse_value(fixture_name) r#"Name of the fixture setup test (default: "fixture")"#,
    -A --arg [arg]        append_value_raw(fixture_args) "Pass an argument to the fixture test binary (can be used multiple times)",
    -x --exec [args...]   take_remaining(exec) "Instead of running cargo test [args...], run the specified command and pass it all remaining arguments",
    -L [level]            parse_value(log_level) "Stderr logging level (choices: off, info, debug, trace, default: info)",
    -h --help             help "Print help",
    --version             version "Print version",
);

def_flags!(
    CARGO_FLAGS:

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
    -p --package [SPEC]      forward_value(cargo_common_test),
    -j --jobs [N]            forward_value(cargo_common_test),
    -r --release             forward(cargo_common_test),
    --profile [NAME]         forward_value(cargo_common_test),
    --target [TRIPLE]        forward_value(cargo_common_test),
    --target-dir [DIR]       forward_value(cargo_common_test),
    --unit-graph             forward(cargo_common_test),
    --timings [FORMATS]      forward_value(cargo_common_test),
);

#[derive(Debug)]
pub struct Cli {
    pub fixture_name: String,
    pub fixture_args: Vec<OsString>,
    pub exec: Vec<OsString>,
    pub log_level: LogLevel,
    pub cargo_common_all: Vec<OsString>,
    pub cargo_common_test: Vec<OsString>,
    pub cargo_test_args: Vec<OsString>,
    pub harness_args: Vec<OsString>,
}

impl Cli {
    fn unknown_flag(&mut self, flag: OsString) {
        self.cargo_test_args.push(flag);
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            fixture_name: "fixture".to_string(),
            fixture_args: vec![],
            exec: vec![],
            log_level: LogLevel::default(),
            cargo_common_all: vec![],
            cargo_common_test: vec![],
            cargo_test_args: vec![],
            harness_args: vec![],
        }
    }
}

pub fn parse() -> Result<Cli> {
    Parser::new(FLAGS, CARGO_FLAGS, env::args_os())
        .parse()
        .map_err(|err| {
            let usage = Parser::usage();
            if err.severity() == 0 {
                println!("{err}");
            } else {
                eprintln!("Error: {err}\nUsage: {usage}\nFor more information, try --help");
            }
            process::exit(err.severity());
        })
}
