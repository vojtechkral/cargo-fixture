use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    mem,
    str::FromStr,
};

use os_str_bytes::RawOsStr;
use tabular::{row, Table};
use thiserror::Error;

use super::{flags::FlagDef, Cli, FLAGS};
use crate::utils::OsStrExt as _;

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type ParseFn = dyn Fn(&mut Parser) -> Result<()> + Send + Sync + 'static;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Flag not convertible to unicode: {0}")]
    NonUnicodeFlag(String),

    #[error("Unrecognized flag: {0}")]
    UnrecognizedFlag(String),

    #[error("Flag {0} doesn't expect a value")]
    UnexpectedValue(String),

    #[error("Flag {0} requires a value")]
    MissingValue(String),

    #[error("Error parsing value for flag {flag}")]
    ParseError {
        flag: String,
        error: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("{0}")]
    Help(String),
    #[error("{0}")]
    Version(String),
}

impl Error {
    pub fn severity(&self) -> i32 {
        match self {
            Error::Help(_) | Error::Version(_) => 0,
            _ => 1,
        }
    }
}

trait OptionExt<T> {
    fn or_non_unicode_flag(self, flag: impl Into<String>) -> Result<T>;
    fn or_unrecognized_flag(self, flag: impl Into<String>) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_non_unicode_flag(self, flag: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| Error::NonUnicodeFlag(flag.into()))
    }
    fn or_unrecognized_flag(self, flag: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| Error::UnrecognizedFlag(flag.into()))
    }
}

trait ResultExt<T, E> {
    fn parser_error(self, flag: impl Into<String>) -> Result<T>;
}

impl<T, E> ResultExt<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn parser_error(self, flag: impl Into<String>) -> Result<T> {
        self.map_err(|err| Error::ParseError {
            flag: flag.into(),
            error: Box::new(err),
        })
    }
}

#[derive(Debug)]
struct RawFlag {
    short: bool,
    name: String,
    eq_value: Option<OsString>,
}

impl RawFlag {
    fn long(name: impl Into<String>, eq_value: Option<OsString>) -> Self {
        Self {
            short: false,
            name: name.into(),
            eq_value,
        }
    }
    fn short(name: impl Into<String>, eq_value: Option<OsString>) -> Self {
        Self {
            short: true,
            name: name.into(),
            eq_value,
        }
    }
    fn empty() -> Self {
        Self {
            short: false,
            name: String::new(),
            eq_value: None,
        }
    }
}

impl From<RawFlag> for OsString {
    fn from(this: RawFlag) -> Self {
        let dashes = if this.short { "-" } else { "--" };
        let name = this.name;
        if let Some(eq_value) = this.eq_value {
            let mut os = OsString::from(format!("{dashes}{name}="));
            os.push(eq_value);
            os
        } else {
            OsString::from(format!("{dashes}{name}"))
        }
    }
}

#[derive(Debug)]
enum NormalizedArg {
    Prog(OsString),
    CargoExt(OsString),
    Flag(RawFlag),
    Positional(OsString),
    Delimiter, // ie. --
}

impl From<NormalizedArg> for OsString {
    fn from(this: NormalizedArg) -> Self {
        match this {
            NormalizedArg::Prog(p) => p,
            NormalizedArg::CargoExt(e) => e,
            // NormalizedArg::ShortFlag { flag, eq_value: Some(v) } => {
            //     let mut os = OsString::from(format!("{flag}="));
            //     os.push(v);
            //     os
            // },
            // NormalizedArg::ShortFlag { flag, .. } => {
            //     OsString::from(format!("{flag}"))
            // },
            // NormalizedArg::LongFlag { flag, eq_value: Some(v) } => {
            //     let mut os = OsString::from(format!("{flag}="));
            //     os.push(v);
            //     os
            // },
            // NormalizedArg::LongFlag { flag, .. } => OsString::from(flag),
            NormalizedArg::Flag(f) => f.into(),
            NormalizedArg::Positional(arg) => arg,
            NormalizedArg::Delimiter => OsString::from("--"),
        }
    }
}

trait Normalize {
    fn normalize(self) -> Result<VecDeque<NormalizedArg>>;
}

impl<I> Normalize for I
where
    I: IntoIterator<Item = OsString>,
{
    fn normalize(self) -> Result<VecDeque<NormalizedArg>> {
        let mut deq = VecDeque::new();
        let mut iter = self.into_iter();
        let Some(prog) = iter.next() else {
            return Ok(deq);
        };
        let cargo_ext = prog
            .to_string_lossy()
            .rsplit_once('-')
            .map(|(_, ext)| ext.to_string());
        deq.push_back(NormalizedArg::Prog(prog));

        let mut iter = iter.peekable();
        if let Some(cargo_ext) = cargo_ext {
            if let Some(cargo_ext) = iter.next_if(|arg| arg.as_os_str() == cargo_ext.as_str()) {
                deq.push_back(NormalizedArg::CargoExt(cargo_ext));
            }
        }

        iter.try_fold(deq, |mut deq, arg| {
            if !arg.as_os_str().starts_with('-') || arg == "-" {
                deq.push_back(NormalizedArg::Positional(arg));
                return Ok(deq);
            }

            if arg == "--" {
                deq.push_back(NormalizedArg::Delimiter);
                return Ok(deq);
            }

            // Positionals, "--", and "-" taken care of...

            let flag = RawOsStr::new(&arg);
            let flag = &*flag;
            // Try to extract a value passed using the --flag=value syntax:
            let (flag, eq_value) = flag
                .split_once("=")
                .map(|(flag, eq_value)| (flag, Some(eq_value.to_os_str().into_owned())))
                .unwrap_or((flag, None));

            // The flag part needs to be unicode
            let flag = flag.to_str().or_non_unicode_flag(flag.to_str_lossy())?;
            if flag.starts_with("--") {
                // Long flag
                // This also takes stuff like ---foo or --- but that's ok
                deq.push_back(NormalizedArg::Flag(RawFlag::long(
                    flag.strip_prefix("--").unwrap(),
                    eq_value,
                )));
                return Ok(deq);
            }

            // Cluster of short flags
            let flags = flag.strip_prefix('-').unwrap();
            let (flags, last) = flags.split_at(flags.len() - 1);

            for f in flags.chars() {
                deq.push_back(NormalizedArg::Flag(RawFlag::short(f, None)));
            }

            // Last flag takes the value, if any
            deq.push_back(NormalizedArg::Flag(RawFlag::short(
                last.chars().next().unwrap(),
                eq_value,
            )));
            Ok(deq)
        })
    }
}

#[derive(Debug)]
pub struct Parser {
    // Flag definitions
    shorts: HashMap<&'static str, &'static FlagDef>,
    longs: HashMap<&'static str, &'static FlagDef>,

    // Input
    args: VecDeque<NormalizedArg>,

    // Parsing state
    current_flag: RawFlag,
    current_flag_def: &'static FlagDef,
    delimiter_found: bool,

    // Result
    cli: Cli,
}

impl Parser {
    pub fn new(
        flags: impl IntoIterator<Item = &'static FlagDef> + Clone,
        args: impl IntoIterator<Item = OsString>,
    ) -> Result<Self> {
        let flags2 = flags.clone();
        Ok(Self {
            shorts: flags
                .into_iter()
                .filter(|f| f.short.is_some())
                .map(|f| (f.short.unwrap(), f))
                .collect(),
            longs: flags2
                .into_iter()
                .filter(|f| f.long.is_some())
                .map(|f| (f.long.unwrap(), f))
                .collect(),
            args: args.normalize()?,
            current_flag: RawFlag::empty(), // dummy value
            current_flag_def: &FlagDef::EMPTY,
            delimiter_found: false,
            cli: Cli::default(),
        })
    }

    pub fn parse(mut self) -> Result<Cli> {
        while let Some(arg) = self.args.pop_front() {
            let flag = match arg {
                NormalizedArg::Prog(_) => continue,
                NormalizedArg::CargoExt(_) => continue,
                NormalizedArg::Flag(flag) if self.delimiter_found => {
                    self.cli.harness_args.push(flag.into());
                    continue;
                }
                NormalizedArg::Flag(flag) => flag,
                NormalizedArg::Positional(arg) => {
                    if !self.delimiter_found {
                        self.cli.cargo_test_args.push(arg);
                    } else {
                        self.cli.harness_args.push(arg);
                    }
                    continue;
                }
                NormalizedArg::Delimiter => {
                    self.delimiter_found = true;
                    continue;
                }
            };

            let def_map = if flag.short {
                &self.shorts
            } else {
                &self.longs
            };
            let flag_def = def_map
                .get(flag.name.as_str())
                .or_unrecognized_flag(&flag.name)?;

            self.current_flag = flag;
            self.current_flag_def = flag_def;
            (flag_def.parse_fn)(&mut self)?;

            if self.current_flag.eq_value.is_some() {
                return Err(Error::UnexpectedValue(self.take_current_flag().name));
            }
        }

        Ok(self.cli)
    }

    fn take_current_flag(&mut self) -> RawFlag {
        mem::replace(&mut self.current_flag, RawFlag::empty())
    }

    fn missing_value_error(&mut self) -> Error {
        Error::MissingValue(self.take_current_flag().name)
    }

    fn get_value(&mut self) -> Result<OsString> {
        let value = self.get_value_raw()?;
        if value.as_os_str().starts_with('-') {
            Err(self.missing_value_error())
        } else {
            Ok(value)
        }
    }

    fn get_value_raw(&mut self) -> Result<OsString> {
        if let Some(value) = self.current_flag.eq_value.take() {
            return Ok(value);
        }

        self.args
            .pop_front()
            .and_then(|arg| match arg {
                NormalizedArg::Positional(arg) => Some(arg),
                _ => None,
            })
            .ok_or_else(|| self.missing_value_error())
    }

    // flag parse fns

    pub fn parse_value<T>(&mut self, field: impl Fn(&mut Cli) -> &mut T) -> Result<()>
    where
        T: FromStr,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let value = self.get_value()?;
        let value = value
            .to_str()
            .or_unrecognized_flag(self.take_current_flag().name)?;
        let value = value
            .parse::<T>()
            .parser_error(self.take_current_flag().name)?;
        *field(&mut self.cli) = value;
        Ok(())
    }

    pub fn append_value_raw(
        &mut self,
        field: impl Fn(&mut Cli) -> &mut Vec<OsString>,
    ) -> Result<()> {
        let value = self.get_value_raw()?;
        let field = field(&mut self.cli);
        field.push(value);
        Ok(())
    }

    pub fn take_remaining(&mut self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Result<()> {
        let field = field(&mut self.cli);
        field.extend(self.args.drain(..).map(OsString::from));
        Ok(())
    }

    pub fn forward(&mut self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Result<()> {
        let flag = self.take_current_flag();
        field(&mut self.cli).push(flag.into());
        Ok(())
    }

    pub fn forward_value(&mut self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> Result<()> {
        let flag = self.take_current_flag();
        let has_eq_value = flag.eq_value.is_some();
        field(&mut self.cli).push(flag.into());

        if !has_eq_value {
            let value = self.get_value_raw()?;
            field(&mut self.cli).push(value.into());
        }

        Ok(())
    }

    pub fn help(&mut self) -> Result<()> {
        Err(Error::Help(Self::build_help()))
    }

    pub fn version(&mut self) -> Result<()> {
        let ver = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        Err(Error::Version(ver))
    }

    // help utils

    pub fn usage() -> String {
        let name = env!("CARGO_PKG_NAME").replace('-', " ");
        format!("{name} [options...] [cargo test args...] [-- test binary args...]")
    }

    fn build_help() -> String {
        let usage = Self::usage();
        let mut help = format!(
            r#"{usage}

Arguments:
  [cargo test args...]   Arguments passed to cargo test.
  [test binary args...]  Arguments passed to the test binary via cargo test -- args...

Options:
"#
        );

        let table = Table::new("  {:<}  {:<}");
        let table = FLAGS
            .iter()
            .filter(|flag| !flag.help.is_empty())
            .fold(table, |table, flag| {
                table.with_row(row!(flag.help_def(), flag.help))
            });
        help.push_str(&format!("{table}"));

        help.push_str("\n  Additionally, the following cargo options are recognized and passed to all cargo calls as appropriate:\n");
        // let table = Table::new("  {:<} {:<}");
        // let cargo_defs = FLAGS.iter().filter(|flag| flag.help.is_empty());
        // let table = FLAGS
        //     cargo_defs.take(10).zip(car)
        //     .fold(table, |table, flag| {
        //         table.with_row(row!(flag.help_def(), flag.help))
        //     });
        // help.push_str(&format!("{table}"));

        help
    }
}

// [off, info, debug, trace]
