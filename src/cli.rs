use std::{
    env,
    ffi::{OsStr, OsString},
    iter,
    process::{Command, Stdio},
};

use clap::{value_parser, Arg, ArgMatches, FromArgMatches, Parser};
use os_str_bytes::RawOsStr;

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
#[command(multicall = true)]
enum Commands {
    #[command(name = "cargo fixture", version)]
    Fixture(Cli),
}

#[derive(Parser, Debug)]
pub struct Cli {
    #[clap(skip = Self::cargo_path())]
    cargo: OsString,

    /// Pass flags or arguments to the fixture binary
    #[arg(short = 'A', value_name = "FLAG|ARG", allow_hyphen_values = true)]
    fixture_args: Vec<String>,

    /// Set stderr verbosity
    #[arg(short, action = clap::ArgAction::Count, default_value_t = 0)]
    verbosity: u8,

    /// No stderr logging
    #[arg(short, default_value_t = false)]
    quiet: bool,

    /// Instead of running cargo test [args...] run the specified command and pass it all remaining arguments
    #[arg(short = 'x', allow_hyphen_values = true, num_args = 1.., value_name = "ARGS")]
    exec: Vec<OsString>,

    #[clap(flatten)]
    args: Args,
}

impl Cli {
    pub fn verbosity(&self) -> u32 {
        if self.quiet {
            0
        } else {
            self.verbosity as u32 + 1
        }
    }

    pub fn fixture_cmd(&self) -> Command {
        let mut cmd = Command::new(self.cargo.clone());
        cmd.args(&self.args.common_cargo_flags)
            .args(["test", "--test", "fixture"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        cmd
    }

    pub fn test_cmd(&self) -> Command {
        let mut cmd = if let Some(exec) = self.exec.get(0) {
            let mut cmd = Command::new(exec);
            cmd.args(&self.exec[1..]);
            cmd
        } else {
            let mut cmd = Command::new(self.cargo.clone());
            cmd.args(["test", "--features", "fixture"]); // FIXME: additive features, // FIXME: configurable feature?
            cmd.args(&self.args.args);
            cmd
        };
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        cmd
    }

    fn cargo_path() -> OsString {
        env::var_os("CARGO").unwrap_or_else(|| {
            env::set_var("CARGO", "cargo");
            "cargo".to_string().into()
        })
    }
}

#[derive(Debug)]
struct Args {
    common_cargo_flags: Vec<OsString>,
    args: Vec<OsString>,
}

impl clap::Args for Args {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd.arg(
            Arg::new("args")
                .num_args(1..)
                .trailing_var_arg(true)
                .allow_hyphen_values(true)
                .value_parser(value_parser!(OsString))
                .help("cargo test flags/arguments or any command if -x is used"),
        )
        .after_help(get_common_cargo_flags_help())
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_args(cmd)
    }
}

impl FromArgMatches for Args {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut this = Self {
            common_cargo_flags: vec![],
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
        self.common_cargo_flags = get_common_cargo_flags(&self.args);
        Ok(())
    }
}

fn get_common_cargo_flags(args: &[OsString]) -> Vec<OsString> {
    let mut res = Vec::with_capacity(args.len());

    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if let Some(take_next_arg) = COMMON_CARGO_FLAGS.iter().find_map(|flag| flag.matches(arg)) {
            res.push(arg.clone());
            if take_next_arg {
                args.next().map(|arg| res.push(arg.clone()));
            }
        }
    }

    res
}

fn get_common_cargo_flags_help() -> String {
    let help = "The following cargo flags are passed to all invocations of cargo (not just the main cargo test call):\n".to_string();
    COMMON_CARGO_FLAGS.iter().fold(help, |mut help, flag| {
        help.push_str(&format!("  {}", flag.name));
        flag.name2.map(|name2| help.push_str(&format!(", {name2}")));
        flag.value_name
            .map(|value_name| help.push_str(&format!(" {value_name}")));
        help.push('\n');
        help
    })
}

#[derive(Debug)]
struct CommonCargoFlag {
    name: &'static str,
    name2: Option<&'static str>,
    value_name: Option<&'static str>,
}

impl CommonCargoFlag {
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

macro_rules! flag {
    ($name:literal) => {
        CommonCargoFlag {
            name: $name,
            name2: None,
            value_name: None,
        }
    };
    ($name:literal $name2:literal) => {
        CommonCargoFlag {
            name: $name,
            name2: Some($name2),
            value_name: None,
        }
    };
    ($name:literal [ $value_name:literal ]) => {
        CommonCargoFlag {
            name: $name,
            name2: None,
            value_name: Some($value_name),
        }
    };
    ($name:literal $name2:literal [ $value_name:literal ]) => {
        CommonCargoFlag {
            name: $name,
            name2: Some($name2),
            value_name: Some($value_name),
        }
    };
}

static COMMON_CARGO_FLAGS: &[CommonCargoFlag] = &[
    flag!("-q" "--quiet"),
    flag!("-v" "--verbose"),
    flag!("--config"["KEY=VALUE"]),
    flag!("-Z"["FLAG"]),
    flag!("-p" "--package" ["SPEC"]),
    flag!("-F" "--features" ["FEATURES"]),
    flag!("-j" "--jobs" ["N"]),
    flag!("-r" "--release"),
    flag!("--profile"["PROFILE-NAME"]),
    flag!("--target"["TRIPLE"]),
    flag!("--target-dir"["DIRECTORY"]),
    flag!("--unit-graph"),
    flag!("--timings"["FMTS"]),
    flag!("--manifest-path"["PATH"]),
    flag!("--frozen"),
    flag!("--locked"),
    flag!("--offline"),
];

#[cfg(test)]
mod tests {
    use super::*;

    // fn os(s: &str) -> OsString {
    //     s.into()
    // }

    fn os<const N: usize>(str_array: [&'static str; N]) -> [OsString; N] {
        str_array.map(OsString::from)
    }

    #[test]
    fn common_cargo_flags() {
        let common = get_common_cargo_flags(&os([
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
