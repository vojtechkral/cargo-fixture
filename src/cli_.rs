use std::{ffi::{OsString, OsStr}, env, iter};

use bpaf::{Bpaf, Parser, short, construct, doc::Style};

use crate::logger::LogLevel;

pub fn parse() -> Cli_ {
    // FIXME: explain
    // let mut args = env::args_os().skip(1).peekable();
    // let arg0 = OsString::from("cargo fixture");
    // if args.peek().map(|arg| arg.as_os_str()) == Some(OsStr::new("fixture")) {
    //     args.next().unwrap();
    //     // TODO: test this
    // }
    // let args: Vec<_> = iter::once(arg0).chain(args).collect();
    // match Commands::parse_from(args) {
    //     Commands::Fixture(fixture) => fixture,
    // }
    // cli_().run_inner(&args[..]).expect("FIXME:")
    cli_().run()
}

#[derive(Bpaf, Debug)]
#[bpaf(options("fixture"))]
pub struct Cli_ {

    /// Set stderr logging level
    /* value_enum, default_value_t = LogLevel::Info */
    // #[bpaf(external, short('L'))]
    #[bpaf(short('L'), fallback(LogLevel::Info))]
    pub log_level: LogLevel,

    /// Pass a flag/argument to the fixture binary; use multiple times to pass several arguments
    // #[bpaf(short = 'A', value_name = "FLAG|ARG", allow_hyphen_values = true)]
    // #[bpaf(short('A'), any("REST", Some), many)]
    // #[bpaf(any("REST", Some), many)]
    // pub fixture_args: OsString,
    // pub fixture_args: Vec<OsString>,
    // #[bpaf(short('A'), argument("FLAG|ARG"))]
    #[bpaf(external(fixture_arg), many, help("pokus"))]
    pub fixture_args: Vec<OsString>,
    // pub fixture_args: Vec<FixtureArg>,

    // // TODO: keep fixture data flag?
    // /// Instead of running cargo test [args...] run the specified command and pass it all remaining arguments
    // #[bpaf(short = 'x', allow_hyphen_values = true, num_args = 1.., value_name = "ARGS")]
    // pub exec: Vec<OsString>,

    // /// Print version
    // #[bpaf(long, action = ArgAction::Version)]
    // version: (),

    // #[clap(flatten)]
    // pub args: Args,
}

// fn fixture_args() -> impl Parser<Vec<OsString>> {
//     short('A')
//         .argument("FLAG|ARG")
//         .any()
//         .many()
// }

fn fixture_arg() -> impl Parser<OsString> {
    // let flag = bpaf::short('A').help("help on flag").req_flag(());
    let flag = bpaf::short('A').help("help on flag").req_flag("hm?");
    let value = bpaf::any("FLAG|ARG", Some).hide();


    let anybased = construct!(flag, value)
        .map(|(_, v)| v)
        .custom_usage("-A arg|--flag").hide();

    let alt = bpaf::short('A').help("alt").argument::<OsString>("ARG");
    construct!([alt, anybased])
}

// #[derive(Bpaf, Debug)]
// #[bpaf(adjacent)]
// pub struct FixtureArg_ {
//     // doc comment?
//     #[bpaf(short('A'))]
//     flag: (),

//     // doc comment?
//     #[bpaf(any("FLAG|ARG", Some))]
//     arg: OsString,
// }
