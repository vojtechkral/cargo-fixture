// Stav: bpafem by to šlo, jsou použitý nějaký nehezký triky, a bylo by potřeba dodělat
// editování helpu (nahrazení '=') a parsing common cargo flags.
// Otázka je, jestli to má smysl, když je to potřeba až tak hodně ohýbat.

use std::{
    cell::Cell,
    env,
    ffi::{OsStr, OsString},
    iter, process,
};

use bpaf::{batteries::get_usage, construct, doc::Style, short, Bpaf, Parser};

use crate::logger::LogLevel;

pub fn parse() -> Cli_ {
    let before_double_dash = Cell::new(true);
    let (args, harness_args) = env::args_os()
        .filter(|arg| {
            if arg == "--" {
                before_double_dash.set(false);
                false
            } else {
                true
            }
        })
        .partition::<Vec<_>, _>(|_| before_double_dash.get());

    cli_()
        .run_inner(&args[..])
        .map_err(|err| {
            process::exit(err.exit_code());
        })
        .unwrap()
}

// NB. The `options("fixture")` annotation takes care of the optional cargo-generated first argument - extension name.
#[derive(Bpaf, Debug)]
#[bpaf(options("fixture"), version)]
pub struct Cli_ {
    /// Set stderr logging level
    #[bpaf(short('L'), fallback(LogLevel::Info))]
    pub log_level: LogLevel,

    #[bpaf(external(fixture_arg), many)]
    pub fixture_args: Vec<OsString>,

    #[bpaf(external(exec_args))]
    pub exec: Vec<OsString>,

    #[bpaf(
        any("ARGS", not_auto_flags),
        many,
        hide,
        custom_usage("[cargo test args...]")
    )]
    pub cargo_args: Vec<OsString>,
}

/// Parse one `-A arg/flag` argument.
fn fixture_arg() -> impl Parser<OsString> {
    // This parser parses the -A --flag form
    let a = bpaf::short('A').req_flag("");
    let arg = bpaf::any("ARG", Some).hide();
    let flag = construct!(a, arg).map(|t| t.1).hide();

    // This one parses the -A arg form
    let arg = bpaf::short('A')
        .help("Pass a flag or argument to the fixture binary (may be used multiple times)")
        .argument::<OsString>("FLAG|ARG");

    // Parses -A --flag or, if that didn't work, -A arg
    construct!([arg, flag])
}

fn exec_args() -> impl Parser<Vec<OsString>> {
    // HACK:
    // This flag is defined only to generate the right help - with a meta arg,
    // the real flag can't have a meta argument.
    // This flag never parses due to the guard.
    let fake_flag = bpaf::short('x')
        .argument::<OsString>("ARGS...")
        .help(
            "Run the specified command, instead of cargo test, passing it all remaining arguments",
        )
        .guard(|_| false, "")
        .map(|_| ());

    let actual_flag = bpaf::short('x').req_flag("").map(|_| ()).hide();

    let args = bpaf::any("ARGS", Some).some("").hide();
    let flag = construct!([fake_flag, actual_flag]);
    construct!(flag, args).map(|t| t.1).fallback(vec![])
}

/// bpaf doesn't exclude auto-flags like --version or --help from `any`.
fn not_auto_flags(arg: OsString) -> Option<OsString> {
    const auto_flags: &[&str] = &["-V", "--version", "-h", "--help"];
    if auto_flags.iter().any(|&f| arg == f) {
        None
    } else {
        Some(arg)
    }
}
