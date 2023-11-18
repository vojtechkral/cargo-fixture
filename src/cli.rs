use std::{
    env,
    ffi::{OsStr, OsString},
    iter,
};

use clap::{builder::ValueParser, Arg, ArgMatches, FromArgMatches, Parser, value_parser};
use os_str_bytes::RawOsStr;

#[derive(Parser)]
#[command(multicall = true)]
enum Commands {
    #[command(name = "cargo fixture", version)]
    Fixture(Fixture),
}

#[derive(Parser, Debug)]
pub struct Fixture {
    /// Pass flags or arguments to the fixture binary
    #[arg(short = 'A', value_name = "FLAG|ARG", allow_hyphen_values = true)]
    fixture_args: Vec<String>,

    /// Instead of running cargo test [args...] run args... as a general command
    #[arg(short = 'x')]
    exec: bool,
    // FIXME: ^ requires non-empty args check

    /// Set stderr verbosity
    #[arg(short, action = clap::ArgAction::Count, default_value_t = 0)]
    verbosity: u8,

    /// No stderr logging
    #[arg(short, default_value_t = false)]
    quiet: bool,



    #[clap(flatten)]
    args: Args,



    // /// cargo test flags and arguments
    // #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    // cargo_args: Vec<String>,  // FIXME: OsString?


    // #[clap(flatten)]
    // cargo_flags: CargoFlags_,

    // // FIXME: remove - can't easily serialize back into cli
    // #[clap(flatten)]
    // cargo_flags: CargoFlags,

    // #[clap(skip)]
    // common_cargo_flags: Vec<OsString>,
}

pub fn parse() -> Fixture {
    // FIXME: explain
    let mut args = env::args_os().skip(1);
    let mut arg0 = OsString::from("cargo ");
    arg0.push(args.next().unwrap()); // FIXME: err handling / print help ???
    let args = iter::once(arg0).chain(args);
    match Commands::parse_from(args) {
        Commands::Fixture(fixture) => fixture,
    }
}

#[derive(Debug)]
struct Args {
    common_cargo_flags: Vec<OsString>,
    args: Vec<OsString>,
}

impl clap::Args for Args {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd
        .arg(Arg::new("args")
            .num_args(1..)
            .trailing_var_arg(true)
            .allow_hyphen_values(true)
            .value_parser(value_parser!(OsString))
            .help("cargo test flags/arguments or any command if -x is used")
        )
        .after_help(get_common_cargo_flags_help())
    }

    // FIXME:
    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        todo!()
    }
}

impl FromArgMatches for Args {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut this = Self { common_cargo_flags: vec![], args: vec![] };
        this.update_from_arg_matches(matches)?;
        Ok(this)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        let args = matches.get_many::<OsString>("args").into_iter().flatten().map(Clone::clone);
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
        flag.value_name.map(|value_name| help.push_str(&format!(" {value_name}")));
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
        let (arg, value_by_eq) = arg.split_once("=").map(|(a, _)| (a, true)).unwrap_or((arg.as_ref(), false));
        if arg == self.name || self.name2.map(|n| n == arg).unwrap_or(false) {
            Some(self.value_name.is_some() && !value_by_eq)
        } else {
            None
        }
    }
}

macro_rules! flag {
    ($name:literal) => {
        CommonCargoFlag { name: $name, name2: None, value_name: None }
    };
    ($name:literal $name2:literal) => {
        CommonCargoFlag { name: $name, name2: Some($name2), value_name: None }
    };
    ($name:literal [ $value_name:literal ]) => {
        CommonCargoFlag { name: $name, name2: None, value_name: Some($value_name) }
    };
    ($name:literal $name2:literal [ $value_name:literal ]) => {
        CommonCargoFlag { name: $name, name2: Some($name2), value_name: Some($value_name) }
    };
}

static COMMON_CARGO_FLAGS: &[CommonCargoFlag] = &[
    flag!("-q" "--quiet"),
    flag!("-v" "--verbose"),
    flag!("--config" ["KEY=VALUE"]),
    flag!("-Z" ["FLAG"]),
    flag!("-p" "--package" ["SPEC"]),
    flag!("-F" "--features" ["FEATURES"]),
    flag!("-j" "--jobs" ["N"]),
    flag!("-r" "--release"),
    flag!("--profile" ["PROFILE-NAME"]),
    flag!("--target"  ["TRIPLE"]),
    flag!("--target-dir" ["DIRECTORY"]),
    flag!("--unit-graph"),
    flag!("--timings" ["FMTS"]),
    flag!("--manifest-path" ["PATH"]),
    flag!("--frozen"),
    flag!("--locked"),
    flag!("--offline"),
];


// /// A `Copy` counterpart to `clap::ArgAction`.
// #[derive(Clone, Copy, Debug)]
// enum ArgAction {
//     Set,
//     SetTrue,
//     Count,
// }

// impl ArgAction {
//     fn configure_arg(self, arg: Arg) -> Arg {
//         match self {
//             Self::Set => arg
//                 .action(clap::ArgAction::Set)
//                 .value_parser(ValueParser::os_string()),
//             Self::SetTrue => arg.action(clap::ArgAction::SetTrue),
//             Self::Count => arg.action(clap::ArgAction::Count),
//         }
//     }
// }

// enum FlagKind {
//     Long(&'static str),
//     Short(char),
//     Both(&'static str, char),
// }

// struct CargoFlag {
//     id: &'static str,
//     long: Option<&'static str>,
//     short: Option<char>,
//     action: ArgAction,
//     help: &'static str,
// }

// impl CargoFlag {
//     fn new(
//         id: &'static str,
//         long: Option<&'static str>,
//         short: Option<char>,
//         action: ArgAction,
//         help: &'static str,
//     ) -> Self {
//         Self {
//             id,
//             long,
//             short,
//             action,
//             help,
//         }
//     }

//     fn clap_arg(&self) -> Arg {
//         let arg = Arg::new(self.id).long(self.long).help(self.help);
//         let arg = self.action.configure_arg(arg);
//         if let Some(short) = self.short {
//             arg.short(short)
//         } else {
//             arg
//         }
//     }

//     fn format_cli(&self) -> OsString {
//         self.long
//             .map((|flag| format!("--{}", flag).into()))
//             .or_else(|| self.short.map(|flag| format!("-{flag}").into()))
//             .expect("Either short or long has to be set.")
//     }

//     // fn format_long(&self) -> Option<OsString> {
//     //     self.long.map((|flag| format!("--{}", flag).into()))
//     // }

//     // fn format_short(&self) -> Option<OsString> {
//     //     self.short.map(|flag| format!("-{flag}").into())
//     // }

//     fn append_from_matches(&self, matches: &mut ArgMatches, append_to: &mut Vec<OsString>) {
//         match self.action {
//             ArgAction::Set => {
//                 if let Some(value) = matches.remove_one::<OsString>(self.long) {
//                     append_to.push(self.format_cli());
//                     append_to.push(value);
//                 }
//             }
//             ArgAction::SetTrue => {
//                 if matches.get_flag(self.id) {
//                     append_to.push(self.format_cli());
//                 }
//             }
//             ArgAction::Count => {
//                 let flag = self.format_short().unwrap_or_else(|| self.format_long());
//                 for _ in 0..matches.get_count(self.long) {
//                     append_to.push(flag.clone());
//                 }
//             }
//         }
//     }
// }

// static CARGO_FLAGS: &[CargoFlag] = &[
//     // CargoFlag::new("quiet", Some('q'), ArgAction::SetTrue, "TODO:")
//     CargoFlag::new(
//         "config",
//         None,
//         ArgAction::Set,
//         "Override a configuration value",
//     ),
//     CargoFlag::new(
//         "Z",
//         None,
//         ArgAction::Set,
//         "Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details",
//     ),
//     // CargoFlag::new(long, short, action, help),
//     //     --config <KEY=VALUE>
//     // -Z <FLAG>                     Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details

//     // Feature Selection:
//     // -p, --package [<SPEC>]  Package to run tests for
//     // TODO:
//     //   -F, --features <FEATURES>  Space or comma separated list of features to activate

//     // Compilation Options:
//     //   -j, --jobs <N>                Number of parallel jobs, defaults to # of CPUs.
//     //   -r, --release                 Build artifacts in release mode, with optimizations
//     //       --profile <PROFILE-NAME>  Build artifacts with the specified profile
//     //       --target [<TRIPLE>]       Build for the target triple
//     //       --target-dir <DIRECTORY>  Directory for all generated artifacts
//     //       --unit-graph              Output build graph in JSON (unstable)
//     //       --timings[=<FMTS>]        Timing output formats (unstable) (comma separated): html, json

//     // Manifest Options:
//     //       --manifest-path <PATH>  Path to Cargo.toml
//     //       --frozen                Require Cargo.lock and cache are up to date
//     //       --locked                Require Cargo.lock is up to date
//     //       --offline               Run without accessing the network
// ];

// #[derive(Debug)]
// struct CargoFlags_(Vec<OsString>);

// impl FromArgMatches for CargoFlags_ {
//     fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
//         Self::from_arg_matches_mut(&mut matches.clone())
//     }

//     fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, clap::Error> {
//         let mut this = Self(vec![]);
//         this.update_from_arg_matches_mut(matches)?;
//         Ok(this)
//     }

//     fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
//         self.update_from_arg_matches_mut(&mut matches.clone())
//     }

//     fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), clap::Error> {
//         for flag in CARGO_FLAGS.iter() {
//             flag.append_from_matches(matches, &mut self.0);
//         }
//         Ok(())
//     }
// }

// impl Args for CargoFlags_ {
//     fn augment_args(cmd: clap::Command) -> clap::Command {
//         CARGO_FLAGS
//             .iter()
//             .fold(cmd, |cmd, flag| cmd.arg(flag.clap_arg()))
//     }

//     fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
//         Self::augment_args(cmd)
//     }
// }



// ---


// -q, --quiet                   Display one character per test instead of one line
// -v, --verbose...              Use verbose output (-vv very verbose/build.rs output)
//     --config <KEY=VALUE>      Override a configuration value
// -Z <FLAG>                     Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details

// Feature Selection:
// -p, --package [<SPEC>]  Package to run tests for
// TODO:
//   -F, --features <FEATURES>  Space or comma separated list of features to activate

// Compilation Options:
//   -j, --jobs <N>                Number of parallel jobs, defaults to # of CPUs.
//   -r, --release                 Build artifacts in release mode, with optimizations
//       --profile <PROFILE-NAME>  Build artifacts with the specified profile
//       --target [<TRIPLE>]       Build for the target triple
//       --target-dir <DIRECTORY>  Directory for all generated artifacts
//       --unit-graph              Output build graph in JSON (unstable)
//       --timings[=<FMTS>]        Timing output formats (unstable) (comma separated): html, json

// Manifest Options:
//       --manifest-path <PATH>  Path to Cargo.toml
//       --frozen                Require Cargo.lock and cache are up to date
//       --locked                Require Cargo.lock is up to date
//       --offline               Run without accessing the network


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
        let common = get_common_cargo_flags(&os(["--foo", "-q", "--package=foo", "--doc", "--no-run", "-p", "foo", "bar", "-Z"]));
        assert_eq!(common, ["-q", "--package=foo", "-p", "foo", "-Z"]);
    }
}
