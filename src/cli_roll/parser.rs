use std::{ffi::OsString, fmt};

use super::Cli;

macro_rules! flags {
    (@flag
        [$($acc:expr,)*]
        $(-$short:ident)?
        $(--$long:ident $(-$long2:ident $(-$long3:ident)?)?)?
        $([$($meta:tt)+])?
        $action:ident $(($field:ident))?
        $($help:literal)?
        , $($tt:tt)*
    ) => {
        flags!(@flag [$($acc,)*
            Flag {
                $( short: Some(flags!(@char $short)), )?
                $( long: Some(concat!(stringify!($long) $(, "-", stringify!($long2) $(, "-", stringify!($long3))?)?)), )?
                $( meta: Some(flags!(@meta $($meta)+)), )?
                $( help: $help, )?

                ..Flag {
                    short: None,
                    long: None,
                    parse_fn: flags!(@action $action $(($field))?),
                    help: "",
                    meta: None,
                }
            },]
            $($tt)*
        );
    };
    (@flag [$($acc:expr,)*]) => {
        // We're done
        pub static FLAGS: &[Flag] = &[
            $($acc,)*
        ];
    };

    // Actions
    (@action parse_value($field:ident)) => { &|parser| { parser.parse_value(|cli| { &mut cli.$field }) } };
    (@action append_value_raw($field:ident)) => { &|parser| { parser.append_value_raw(|cli| { &mut cli.$field }) } };
    (@action forward($field:ident)) => { &|parser| { parser.forward(|cli| { &mut cli.$field }) } };
    (@action forward_value($field:ident)) => { &|parser| { parser.forward_value(|cli| { &mut cli.$field }) } };
    (@action take_remaining($field:ident)) => { &|parser| { parser.take_remaining(|cli| { &mut cli.$field }) } };
    (@action take_tail($field:ident)) => {};
    (@action help) => { &|parser| { parser.help() } };
    (@action version) => { &|parser| { parser.version() } };

    // Parsing of meta args
    (@meta $meta:ident ...) => { concat!(stringify!($meta), "...") };
    (@meta $meta:ident=$meta2:ident) => { concat!(stringify!($meta), "=", stringify!($meta2)) };
    (@meta $meta:ident) => { stringify!($meta) };

    // Util: maps single char ident into char (just the ones I need lol)
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

    // Entry point
    ( $($tt:tt)+ ) => { flags!(@flag [] $($tt)+); };
}
pub(crate) use flags;


pub struct Flag {
    pub short: Option<char>,
    pub long: Option<&'static str>,
    pub parse_fn: &'static (dyn Fn(Parser) -> Parser + Send + Sync + 'static), // TODO: Result
    pub help: &'static str,
    pub meta: Option<&'static str>,
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


#[derive(Debug)]
pub struct Parser {
    dummy: u32,
}

impl Parser {
    pub fn parse_value<T>(self, field: impl Fn(&mut Cli) -> &mut T) -> Self {
        todo!();
        self
    }

    pub fn append_value_raw(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    pub fn take_remaining(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    pub fn forward(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    pub fn forward_value(self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Self {
        todo!();
        self
    }

    pub fn help(self) -> Self {
        todo!();
        self
    }

    pub fn version(self) -> Self {
        todo!();
        self
    }
}