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

    #[clap(flatten)]
    pub args: Args,
}

/// FIXME: explain
#[derive(Debug)]
pub struct Args {
    pub cargo_flags_common: Vec<OsString>,
    pub cargo_flags_test: Vec<OsString>,
    pub args: Vec<OsString>,
}

impl clap::Args for Args {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd.arg(
            Arg::new("args")
                .num_args(1..)
                .trailing_var_arg(true)
                // .action(ArgAction::Append)
                .allow_hyphen_values(true)
                .value_parser(value_parser!(OsString))
                .value_delimiter(None)
                .help("cargo test flags/arguments or any command if -x is used"),
        )
        .after_help({
            let mut after_help = "The following cargo flags are passed to all invocations of cargo:\n".to_string();
            after_help.push_str(&CARGO_FLAGS_ALL.help_str());
            after_help.push_str("\nThe following cargo flags are additionally passed to all invocations of cargo test:\n");
            after_help.push_str(&CARGO_FLAGS_TEST.help_str());
            after_help
        })
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for Args {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut this = Self {
            cargo_flags_common: vec![],
            cargo_flags_test: vec![],
            args: vec![],
        };
        this.update_from_arg_matches(matches)?;
        Ok(this)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        let args = matches
            .get_many::<OsString>("args")
            .into_iter()
            .flatten()
            .map(Clone::clone);
        self.args.extend(args);
        self.cargo_flags_common = CARGO_FLAGS_ALL.filter_args(&self.args);
        self.cargo_flags_test = CARGO_FLAGS_TEST.filter_args(&self.args);
        Ok(())
    }
}

struct CommonCargoFlags(&'static [CommonCargoFlag]);

/// CommonCargoFlags c-tor.
macro_rules! flags {
    ( $(( $($tt:tt)+ ),)+ ) => {
        CommonCargoFlags(&[ $(flags!(@ $($tt)+)),+ ])
    };
    (@ $name:literal) => {
        CommonCargoFlag::new($name, None, None)
    };
    (@ $name:literal, $name2:literal) => {
        CommonCargoFlag::new($name, Some($name2), None)
    };
    (@ $name:literal = $value_name:literal) => {
        CommonCargoFlag::new($name, None, Some($value_name))
    };
    (@ $name:literal, $name2:literal = $value_name:literal) => {
        CommonCargoFlag::new($name, Some($name2), Some($value_name))
    };
}

impl CommonCargoFlags {
    fn filter_args(&self, args: &[OsString]) -> Vec<OsString> {
        let mut res = Vec::with_capacity(args.len());

        let mut args = args.iter();
        while let Some(arg) = args.next() {
            if let Some(take_next_arg) = self.0.iter().find_map(|flag| flag.matches(arg)) {
                res.push(arg.clone());
                if take_next_arg {
                    args.next().map(|arg| res.push(arg.clone()));
                }
            }
        }

        res
    }

    fn help_str(&self) -> String {
        self.0
            .iter()
            .enumerate()
            .fold(" ".to_string(), |mut help, (i, flag)| {
                help.push_str(&format!(" {}", flag.name));
                flag.name2.map(|name2| help.push_str(&format!("/{name2}")));
                flag.value_name
                    .map(|value_name| help.push_str(&format!(" {value_name}")));
                help.push(',');
                if (i + 1) % 4 == 0 {
                    help.push_str("\n ");
                }
                help
            })
    }
}

#[derive(Debug)]
struct CommonCargoFlag {
    name: &'static str,
    name2: Option<&'static str>,
    value_name: Option<&'static str>,
}

impl CommonCargoFlag {
    const fn new(
        name: &'static str,
        name2: Option<&'static str>,
        value_name: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            name2,
            value_name,
        }
    }

    /// FIXME: doc
    fn matches(&self, arg: &OsStr) -> Option<bool> {
        let arg = RawOsStr::new(arg);
        let (arg, value_by_eq) = arg
            .split_once("=")
            .map(|(a, _)| (a, true))
            .unwrap_or((arg.as_ref(), false));
        if arg == self.name || self.name2.map(|n| n == arg).unwrap_or(false) {
            Some(self.value_name.is_some() && !value_by_eq)
        } else {
            None
        }
    }
}

/// Cargo flags common for all used `cargo` subcommands.
static CARGO_FLAGS_ALL: CommonCargoFlags = flags!(
    ("-q", "--quiet"),
    ("-v", "--verbose"),
    ("-Z" = "FLAG"),
    ("--color" = "WHEN"),
    ("--config" = "KEY=VALUE"),
    // Feature Selection:
    ("-F", "--features" = "FEATURES"),
    ("--all-features"),
    ("--no-default-features"),
    // Manifest Options:
    ("--manifest-path" = "PATH"),
    ("--frozen"),
    ("--locked"),
    ("--offline"),
);

/// Cargo flags common for all `cargo test` invocations.
static CARGO_FLAGS_TEST: CommonCargoFlags = flags!(
    ("--ignore-rust-version"),
    ("--future-incompat-report"),
    // Package Selection:   // FIXME: We might need to extract this one too (?) - to get Cargo.toml meta config
    ("-p", "--package" = "SPEC"),
    // Compilation Options:
    ("-j", "--jobs" = "N"),
    ("-r", "--release"),
    ("--profile" = "PROFILE-NAME"),
    ("--target" = "TRIPLE"),
    ("--target-dir" = "DIRECTORY"),
    ("--unit-graph"),
    ("--timings" = "FMTS"),
);

#[cfg(test)]
mod tests {
    use super::*;

    fn os<const N: usize>(str_array: [&'static str; N]) -> [OsString; N] {
        str_array.map(OsString::from)
    }

    #[test]
    fn common_cargo_flags() {
        let common = CARGO_FLAGS_ALL.filter_args(&os([
            "--foo",
            "-q",
            "--package=foo",
            "--doc",
            "--no-run",
            "-p",
            "foo",
            "bar",
            "-Z",
        ]));
        assert_eq!(common, ["-q", "--package=foo", "-p", "foo", "-Z"]);
    }
}
