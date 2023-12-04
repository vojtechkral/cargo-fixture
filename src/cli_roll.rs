use std::{
    ffi::{OsStr, OsString},
    fmt,
};

use anyhow::Result;

use crate::logger::LogLevel;

// TODO: move impl details here
// mod parser;

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

macro_rules! flags {
    ( $({ $($tt:tt)+ })+ ) => {
        pub static FLAGS: &[Flag] = &[
            $(flags!(@start [] $($tt)+ ),)+
        ];
    };

    // for each flag:

    (@start [] --$long:ident $($tt:tt)+ ) => { flags!(@long [None, stringify!($long)] $($tt)+) };
    (@start [] -$short:ident $($tt:tt)+ ) => { flags!(@short [Some(flags!(@char $short))] $($tt)+) };

    (@short [$short:expr] --$long:ident $($tt:tt)+) => {flags!(@long [$short, stringify!($long)] $($tt)+) };
    (@short [$short:expr] $($tt:tt)+) => { flags!(@flags [$short, None] $($tt)+) };

    (@long [$short:expr, $long:expr] -$cont:ident $($tt:tt)+) => {
        flags!(@long [$short, concat!($long, "-", stringify!($cont))] $($tt)+)
    };
    (@long [$short:expr, $long:expr] $($tt:tt)+) => { flags!(@flags [$short, Some($long)] $($tt)+) };

    (@flags [$($args:expr),+] <$meta:ident...> $($tt:tt)+) => {
        flags!(@meta [$($args),+ , Some(concat!(stringify!($meta), "..."))] $($tt)+)
    };
    (@flags [$($args:expr),+] <$meta:ident> $($tt:tt)+) => {
        flags!(@meta [$($args),+ , Some(stringify!($meta))] $($tt)+)
    };
    (@flags [$($args:expr),+] $($tt:tt)+) => { flags!(@meta [$($args),+ , None] $($tt)+) };

    (@meta [$($args:expr),+] $help:literal $($tt:tt)+) => { flags!(@help [$($args),+ , $help] $($tt)+) };
    (@meta [$($args:expr),+] $($tt:tt)+) => { flags!(@help [$($args),+ , ""] $($tt)+) };

    (@help [$($args:expr),+] : $action:ident $($($tt:tt),+)?) => {
        flags!(@action [$($args),+] $action $($($tt),+)?)
    };

    (@action [$($args:expr),+] parse_value($field:ident)) => {
        flags!(@done [$($args),+ , &|parser| { parser.parse_value(|cli| { &mut cli.$field }) }])
    };
    (@action [$($args:expr),+] append_value_raw($field:ident)) => {
        flags!(@done [$($args),+ , &|parser| { parser.append_value_raw(|cli| { &mut cli.$field }) }])
    };
    (@action [$($args:expr),+] forward($field:ident)) => {
        flags!(@done [$($args),+ , &|parser| { parser.forward(|cli| { &mut cli.$field }) }])
    };
    (@action [$($args:expr),+] forward_value($field:ident)) => {
        flags!(@done [$($args),+ , &|parser| { parser.forward_value(|cli| { &mut cli.$field }) }])
    };
    (@action [$($args:expr),+] take_remaining($field:ident)) => {
        flags!(@done [$($args),+ , &|parser| { parser.take_remaining(|cli| { &mut cli.$field }) }])
    };
    (@action [$($args:expr),+] take_tail($field:ident)) => {};
    (@action [$($args:expr),+] help) => { flags!(@done [$($args),+ , &|parser| { parser.help() } ]) };
    (@action [$($args:expr),+] version) => { flags!(@done [$($args),+ , &|parser| { parser.version() } ]) };

    (@done [$short:expr, $long:expr, $meta:expr, $help:expr, $parse_fn:expr]) => {
        Flag {
            short: $short,
            long: $long,
            parse_fn: $parse_fn,
            help: $help,
            meta: $meta,
        }
    };

    // Map single char ident into char
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
    { -L                    "" : parse_value(log_level) }
    { -A                    "" : append_value_raw(fixture_args) }
    { -x --exec <Args...>   "Instead of running cargo test [args...] run the specified command and pass it all remaining arguments" : take_remaining(exec) }
    { -h --help             "" : help }
    { --version             "" : version }

    // Common cargo args
    { -q --quiet                : forward(cargo_common_all) }
    { -v --verbose              : forward(cargo_common_all) }
    { -Z <FLAG>                 : forward_value(cargo_common_all) }
    { --color <WHEN>            : forward_value(cargo_common_all) }
    { --config <KEY_VALUE>      : forward_value(cargo_common_all) }
    { -F --features <FEATURES>  : forward_value(cargo_common_all) }
    { --all-features            : forward(cargo_common_all) }
    { --no-default-features     : forward(cargo_common_all) }
    { --manifest-path <PATH>    : forward_value(cargo_common_all) }
    { --frozen                  : forward(cargo_common_all) }
    { --locked                  : forward(cargo_common_all) }
    { --offline                 : forward(cargo_common_all) }

    /*
    // Common cargo test args
    { --ignore-rust-version    : AppendFlag [Test] }
    { --future-incompat-report : AppendFlag [Test] }
    { -p --package             : AppendFlag [Test] } // TODO: We might need to extract this one too (?) - to get Cargo.toml meta config
    { -j --jobs                : AppendFlag [Test] }
    { -r --release             : AppendFlag [Test] }
    { --profile                : AppendFlag [Test] }
    { --target                 : AppendFlag [Test] }
    { --target-dir             : AppendFlag [Test] }
    { --unit-graph             : AppendFlag [Test] }
    { --timings                : AppendFlag [Test] }
    */
}

#[derive(Debug)]
struct Parser {
    dummy: u32,
}

impl Parser {
    fn parse_value<T>(self, field: impl Fn(&mut Cli) -> &mut T) -> Self {
        todo!();
        self
    }

    fn append_value_raw(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    fn take_remaining(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    fn forward(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    fn forward_value(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    fn help(self) -> Self {
        todo!();
        self
    }

    fn version(self) -> Self {
        todo!();
        self
    }
}

#[derive(Debug)]
enum Metavar {
    None,
    Singular(&'static str),
    Plural(&'static str),
}

// #[derive(Debug)]
// enum Action {
//     /// Takes value, must not start with a `-`.
//     Value(fn(Cli, String) -> Result<Cli>),
//     /// Any value, incl. starting with a `-`.
//     RawValue,
//     /// Take the whole following command line, incl. `-- args...`.
//     TakeAll,
//     /// Take all arguments after `--`.
//     TakeTail,
//     /// `cargo` non-value flag.
//     AppendFlag,
//     /// `cargo` flag with value.
//     AppendRaw,
//     /// Print help and exit.
//     Help,
//     /// Print version info and exit.
//     Version,
// }

#[derive(Debug)]
enum CargoCommonKind {
    All,
    Test,
}

pub struct Flag {
    short: Option<char>,
    long: Option<&'static str>,
    // action: Box<dyn Fn(&mut Cli, &mut Parser, &OsStr) -> Result<()> + Send + Sync + 'static>,
    parse_fn: &'static (dyn Fn(Parser) -> Parser + Send + Sync + 'static), // TODO: Result
    help: &'static str,
    meta: Option<&'static str>,
}

impl fmt::Debug for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            short,
            long,
            parse_fn,
            help,
            meta,
        } = self;
        f.debug_struct("Flag")
            .field("short", &short)
            .field("long", &long)
            .field("parse_fn", &(parse_fn as *const _))
            .field("help", &help)
            .field("meta", &meta)
            .finish()
    }
}
