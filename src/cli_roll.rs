use std::ffi::OsString;

use crate::logger::LogLevel;

// TODO: move impl details here
// mod parser;

pub struct Cli {
    pub fixture_args: Vec<String>,
    pub log_level: LogLevel,
    pub exec: Vec<OsString>,
    pub version: (),
    /// FIXME: doc
    pub cargo_common_all: Vec<OsString>,
    /// FIXME: doc
    pub cargo_common_test: Vec<OsString>,
    pub cargo_test_args: Vec<OsString>,
    pub harness_args: Vec<OsString>,
}

macro_rules! flags {
    ( $({ $($tt:tt)+ })+ ) => {
        pub static FLAGS: &[Flag] = &[
            $(flags!(@start [] $($tt)+ ),)+
        ];
    };

    // for each flag:

    (@start [] --$long:ident $($tt:tt)+ ) => { flags!(@long [None, stringify!($long)] $($tt)+) };
    (@start [] -$short:ident $($tt:tt)+ ) => { flags!(@short [Some(flags!(@char $short))] $($tt)+) };

    (@short [$short:expr] --$long:ident $($tt:tt)+) => { flags!(@long [$short, stringify!($long)] $($tt)+) };
    (@short [$short:expr] $($tt:tt)+) => { flags!(@flags [$short, None] $($tt)+) };

    (@long [$short:expr, $long:expr] -$cont:ident $($tt:tt)+) => { flags!(@long [$short, concat!($long, "-", stringify!($cont))] $($tt)+) };
    (@long [$short:expr, $long:expr] $($tt:tt)+) => { flags!(@flags [$short, Some($long)] $($tt)+) };

    (@flags [$($args:expr),+] $help:literal $($tt:tt)+) => { flags!(@help [$($args),+ , $help] $($tt)+) };
    (@flags [$($args:expr),+] $($tt:tt)+) => { flags!(@help [$($args),+ , ""] $($tt)+) };

    (@help [$($args:expr),+] : $action:ident $($tt:tt)*) => { flags!(@action [$($args),+ , Action::$action] $($tt)*) };

    (@action [$($args:expr),+] [$cargo_kind:ident]) => { flags!(@done [$($args),+ , Some(CargoCommonKind::$cargo_kind)]) };
    (@action [$($args:expr),+]) => { flags!(@done [$($args),+ , None]) };

    (@done [$short:expr, $long:expr, $help:expr, $action:expr, $cargo_kind:expr]) => {
        Flag {
            short: $short,
            long: $long,
            action: $action,
            help: $help,
            cargo_kind: $cargo_kind,
        }
    };

    // Just the ones I need lol
    (@char r) => { 'r' };
    (@char j) => { 'j' };
    (@char p) => { 'p' };
    (@char A) => { 'A' };
    (@char F) => { 'F' };
    (@char h) => { 'h' };
    (@char L) => { 'L' };
    (@char q) => { 'q' };
    (@char v) => { 'v' };
    (@char x) => { 'x' };
    (@char Z) => { 'Z' };
}

flags! {
    // cargo fixture args
    { -L          "" : Value }
    { -A          "" : RawValue }
    { -x --exec   "Instead of running cargo test [args...] run the specified command and pass it all remaining arguments" : TakeAll }
    { -h --help   "" : Help }
    { --version   "" : Version }

    // Common cargo args
    { -q --quiet               : CargoFlag [All] }
    { -v --verbose             : CargoFlag [All] }
    { -Z                       : CargoFlag [All] }
    { --color                  : CargoFlag [All] }
    { --config                 : CargoFlag [All] }
    { -F --features            : CargoFlag [All] }
    { --all-features           : CargoFlag [All] }
    { --no-default-features    : CargoFlag [All] }
    { --manifest-path          : CargoFlag [All] }
    { --frozen                 : CargoFlag [All] }
    { --locked                 : CargoFlag [All] }
    { --offline                : CargoFlag [All] }

    // Common cargo test args
    { --ignore-rust-version    : CargoFlag [Test] }
    { --future-incompat-report : CargoFlag [Test] }
    { -p --package             : CargoFlag [Test] } // TODO: We might need to extract this one too (?) - to get Cargo.toml meta config
    { -j --jobs                : CargoFlag [Test] }
    { -r --release             : CargoFlag [Test] }
    { --profile                : CargoFlag [Test] }
    { --target                 : CargoFlag [Test] }
    { --target-dir             : CargoFlag [Test] }
    { --unit-graph             : CargoFlag [Test] }
    { --timings                : CargoFlag [Test] }
}

#[derive(Debug)]
enum Action {
    /// Does not take value, boolean presence.
    Flag,
    /// Takes value, must not start with a `-`.
    Value,
    /// Any value, incl. starting with a `-`.
    RawValue,
    /// Take the whole following command line, incl. `-- args...`.
    TakeAll,
    /// Take all arguments after `--`.
    TakeTail,
    /// `cargo` non-value flag.
    CargoFlag,
    /// `cargo` flag with value.
    CargoValue,
    /// Print help and exit.
    Help,
    /// Print version info and exit.
    Version,
}

#[derive(Debug)]
enum CargoCommonKind {
    All,
    Test,
}

#[derive(Debug)]
pub struct Flag {
    short: Option<char>,
    long: Option<&'static str>,
    action: Action,
    help: &'static str,
    cargo_kind: Option<CargoCommonKind>,
}
