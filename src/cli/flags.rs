use std::fmt;

use crate::utils::StringExt;

use super::parser::ParseFn;

pub struct FlagDef {
    pub short: Option<&'static str>,
    pub long: Option<&'static str>,
    pub parse_fn: &'static ParseFn,
    pub help: &'static str,
    pub meta: Option<&'static str>,
}

impl FlagDef {
    pub const EMPTY: Self = Self {
        short: None,
        long: None,
        parse_fn: &|_p| Ok(()),
        help: "",
        meta: None,
    };

    pub fn help_def(&self) -> String {
        let mut res = String::new();
        if let Some(s) = self.short {
            res.push_strs(&["-", s])
        }
        if self.short.and(self.long).is_some() {
            res.push_str(", ")
        }
        if let Some(l) = self.long {
            res.push_strs(&["--", l])
        }
        if let Some(m) = self.meta {
            res.push_strs(&[" <", m, ">"])
        }
        res
    }
}

impl fmt::Debug for FlagDef {
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

macro_rules! def_flags {
    (@flag
        $output:ident
        [$($acc:expr,)*]
        $(-$short:ident)?
        $(--$long:ident $(-$long2:ident $(-$long3:ident)?)?)?
        $([$($meta:tt)+])?
        $action:ident $(($field:ident))?
        $($help:literal)?
        , $($tt:tt)*
    ) => {
        def_flags!(@flag $output [$($acc,)*
            #[allow(clippy::needless_update)]
            $crate::cli::flags::FlagDef {
                $( short: Some(stringify!($short)), )?
                $( long: Some(concat!(stringify!($long) $(, "-", stringify!($long2) $(, "-", stringify!($long3))?)?)), )?
                parse_fn: def_flags!(@action $action $(($field))?),
                $( meta: Some(def_flags!(@meta $($meta)+)), )?
                $( help: $help, )?

                ..$crate::cli::flags::FlagDef::EMPTY
            },]
            $($tt)*
        );
    };
    (@flag $output:ident [$($acc:expr,)*]) => {
        // We're done
        pub static $output: &[$crate::cli::flags::FlagDef] = &[
            $($acc,)*
        ];
    };

    // Actions
    (@action set_flag($field:ident)) => { &|parser| { parser.set_flag(|cli| { &mut cli.$field }) } };
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

    // Entry point
    ( $output:ident : $($tt:tt)+ ) => { def_flags!(@flag $output [] $($tt)+); };
}
pub(crate) use def_flags;
