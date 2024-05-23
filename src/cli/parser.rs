use std::{
    collections::{HashMap, VecDeque},
    ffi::OsString,
    mem,
    str::FromStr,
};

use anyhow::{anyhow, Context};
use os_str_bytes::RawOsStr;
use tabular::{row, Table};
use thiserror::Error;

use super::{flags::FlagDef, Cli};
use crate::utils::OsStrExt as _;

pub type ParseResult<T> = std::result::Result<T, Error>;
pub type ParseFn = dyn Fn(&mut Parser) -> ParseResult<()> + Send + Sync + 'static;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Parsing(anyhow::Error),

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

macro_rules! error {
    ( $($tt:tt)+ ) => {
        Error::Parsing(anyhow!($($tt)+))
    };
}

macro_rules! bail {
    ( $($tt:tt)+ ) => {
        return Err(Error::Parsing(anyhow!($($tt)+)))
    };
}

#[derive(Debug)]
struct RawFlag {
    short: bool,
    flag: String, // includes the -/-- prefix
    eq_value: Option<OsString>,
}

impl RawFlag {
    /// `flag` is expected to include the -- prefix
    fn long(flag: impl Into<String>, eq_value: Option<OsString>) -> Self {
        Self {
            short: false,
            flag: flag.into(),
            eq_value,
        }
    }

    fn short(name: char, eq_value: Option<OsString>) -> Self {
        Self {
            short: true,
            flag: format!("-{name}"),
            eq_value,
        }
    }

    fn empty() -> Self {
        Self {
            short: false,
            flag: String::new(),
            eq_value: None,
        }
    }

    fn name(&self) -> &str {
        if self.short {
            &self.flag[1..]
        } else {
            &self.flag[2..]
        }
    }
}

impl From<RawFlag> for OsString {
    fn from(this: RawFlag) -> Self {
        if let Some(eq_value) = this.eq_value {
            let mut os = OsString::from(this.flag);
            os.push("=");
            os.push(eq_value);
            os
        } else {
            OsString::from(this.flag)
        }
    }
}

#[derive(Debug)]
enum NormalizedArg {
    Prog(OsString),
    CargoExt(OsString),
    Flag(RawFlag),
    BadFlag(OsString),
    Positional(OsString),
    Delimiter, // ie. --
}

impl From<NormalizedArg> for OsString {
    fn from(this: NormalizedArg) -> Self {
        match this {
            NormalizedArg::Prog(p) => p,
            NormalizedArg::CargoExt(e) => e,
            NormalizedArg::Flag(f) => f.into(),
            NormalizedArg::BadFlag(f) => f,
            NormalizedArg::Positional(arg) => arg,
            NormalizedArg::Delimiter => OsString::from("--"),
        }
    }
}

trait Normalize {
    fn normalize(self) -> VecDeque<NormalizedArg>;
}

impl<I> Normalize for I
where
    I: IntoIterator<Item = OsString>,
{
    fn normalize(self) -> VecDeque<NormalizedArg> {
        let mut deq = VecDeque::new();
        let mut iter = self.into_iter();
        let Some(prog) = iter.next() else {
            return deq;
        };
        let cargo_ext = prog
            .to_string_lossy()
            .rsplit_once('-')
            .map(|(_, ext)| ext.strip_suffix(".exe").unwrap_or(ext).to_string());
        deq.push_back(NormalizedArg::Prog(prog));

        let mut iter = iter.peekable();
        if let Some(cargo_ext) = cargo_ext {
            if let Some(cargo_ext) = iter.next_if(|arg| arg.as_os_str() == cargo_ext.as_str()) {
                deq.push_back(NormalizedArg::CargoExt(cargo_ext));
            }
        }

        iter.fold(deq, |mut deq, arg| {
            if !arg.as_os_str().starts_with('-') || arg == "-" {
                deq.push_back(NormalizedArg::Positional(arg));
                return deq;
            }

            if arg == "--" {
                deq.push_back(NormalizedArg::Delimiter);
                return deq;
            }

            // Positionals, "--", and "-" taken care of...

            let flag = RawOsStr::new(&arg);
            // Try to extract a value passed using the --flag=value syntax:
            let (flag, eq_value) = flag
                .split_once("=")
                .map(|(flag, eq_value)| (flag, Some(eq_value.as_os_str().to_owned())))
                .unwrap_or((flag, None));

            // The flag part needs to be unicode
            let Some(flag) = flag.to_str() else {
                deq.push_back(NormalizedArg::BadFlag(arg));
                return deq;
            };

            if flag.starts_with("--") {
                // Long flag
                // This also takes stuff like ---foo or --- but that's ok
                deq.push_back(NormalizedArg::Flag(RawFlag::long(flag, eq_value)));
                return deq;
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
            deq
        })
    }
}

#[derive(Debug)]
pub struct Parser {
    // Flag definitions
    flags: &'static [FlagDef],
    cargo_flags: &'static [FlagDef],
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
        flags: &'static [FlagDef],
        cargo_flags: &'static [FlagDef],
        args: impl IntoIterator<Item = OsString>,
    ) -> Self {
        let all_flags = flags.iter().chain(cargo_flags.iter());
        let all_flags2 = all_flags.clone();
        Self {
            flags,
            cargo_flags,
            shorts: all_flags
                .into_iter()
                .filter(|f| f.short.is_some())
                .map(|f| (f.short.unwrap(), f))
                .collect(),
            longs: all_flags2
                .into_iter()
                .filter(|f| f.long.is_some())
                .map(|f| (f.long.unwrap(), f))
                .collect(),
            args: args.normalize(),
            current_flag: RawFlag::empty(), // dummy value
            current_flag_def: &FlagDef::EMPTY,
            delimiter_found: false,
            cli: Cli::default(),
        }
    }

    pub fn parse(mut self) -> ParseResult<Cli> {
        while let Some(arg) = self.args.pop_front() {
            let flag = match arg {
                NormalizedArg::Prog(_) => continue,
                NormalizedArg::CargoExt(_) => continue,
                NormalizedArg::Flag(flag) if self.delimiter_found => {
                    self.cli.harness_args.push(flag.into());
                    continue;
                }
                NormalizedArg::Flag(flag) => flag,
                NormalizedArg::BadFlag(os) => {
                    bail!("Flag not convertible to unicode: {}", os.to_escaped())
                }
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

            let Some(flag_def) = def_map.get(flag.name()) else {
                self.cli.unknown_flag(flag.into());
                continue;
            };

            self.current_flag = flag;
            self.current_flag_def = flag_def;
            (flag_def.parse_fn)(&mut self)?;

            if self.current_flag.eq_value.is_some() {
                bail!(
                    "Flag doesn't take a value: {}",
                    self.take_current_flag().flag
                );
            }
        }

        Ok(self.cli)
    }

    fn take_current_flag(&mut self) -> RawFlag {
        mem::replace(&mut self.current_flag, RawFlag::empty())
    }

    fn missing_value_error(&mut self) -> Error {
        error!("Flag {} requires a value", self.take_current_flag().flag)
    }

    fn get_value(&mut self) -> ParseResult<OsString> {
        let value = self.get_value_raw()?;
        if value.as_os_str().starts_with('-') {
            Err(self.missing_value_error())
        } else {
            Ok(value)
        }
    }

    fn get_value_raw(&mut self) -> ParseResult<OsString> {
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

    pub fn set_flag(&mut self, field: impl Fn(&mut Cli) -> &mut bool) -> ParseResult<()> {
        *field(&mut self.cli) = true;
        Ok(())
    }

    pub fn parse_value<T>(&mut self, field: impl Fn(&mut Cli) -> &mut T) -> ParseResult<()>
    where
        T: FromStr,
        <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
    {
        let value = self.get_value()?;
        let value = value.into_string().map_err(|value| {
            error!(
                "Non-unicode value could not be parsed: {} {}",
                self.take_current_flag().flag,
                value.to_escaped()
            )
        })?;
        let value = value
            .parse::<T>()
            .with_context(|| {
                format!(
                    "Error parsing value for flag: {} {}",
                    self.take_current_flag().flag,
                    value
                )
            })
            .map_err(Error::Parsing)?;
        *field(&mut self.cli) = value;
        Ok(())
    }

    pub fn append_value_raw(
        &mut self,
        field: impl Fn(&mut Cli) -> &mut Vec<OsString>,
    ) -> ParseResult<()> {
        let value = self.get_value_raw()?;
        let field = field(&mut self.cli);
        field.push(value);
        Ok(())
    }

    pub fn take_remaining(
        &mut self,
        field: impl Fn(&mut Cli) -> &mut Vec<OsString>,
    ) -> ParseResult<()> {
        let field = field(&mut self.cli);
        if self.args.is_empty() {
            Err(self.missing_value_error())
        } else {
            field.extend(self.args.drain(..).map(OsString::from));
            Ok(())
        }
    }

    pub fn forward(&mut self, field: impl Fn(&mut Cli) -> &mut Vec<OsString>) -> ParseResult<()> {
        let flag = self.take_current_flag();
        field(&mut self.cli).push(flag.into());
        Ok(())
    }

    pub fn forward_value(
        &mut self,
        field: impl Fn(&mut Cli) -> &mut Vec<OsString>,
    ) -> ParseResult<()> {
        let flag = self.take_current_flag();
        let has_eq_value = flag.eq_value.is_some();
        field(&mut self.cli).push(flag.into());

        if !has_eq_value {
            let value = self.get_value_raw()?;
            field(&mut self.cli).push(value);
        }

        Ok(())
    }

    pub fn help(&mut self) -> ParseResult<()> {
        Err(Error::Help(self.build_help()))
    }

    pub fn version(&mut self) -> ParseResult<()> {
        let ver = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        Err(Error::Version(ver))
    }

    // help utils

    pub fn usage() -> String {
        let name = env!("CARGO_PKG_NAME").replace('-', " ");
        format!("{name} [options...] [cargo test opts/args...] [-- test binary args...]")
    }

    fn build_help(&self) -> String {
        let usage = Self::usage();
        let mut help = format!(
            r#"{usage}

Arguments:
  [cargo test opts/args...]   Arguments passed to cargo test.
  [test binary args...]       Arguments passed to the test binary via cargo test [...] -- args...

Options:
"#
        );

        let table = Table::new("  {:<}  {:<}");
        let table = self.flags.iter().fold(table, |table, flag| {
            table.with_row(row!(flag.help_def(), flag.help))
        });
        help.push_str(&format!("{table}"));

        help.push_str("\nAdditionally, the following cargo options are recognized and passed to all cargo calls as appropriate:\n");
        let third = self.cargo_flags.len() / 3;
        let (p1, rest) = self.cargo_flags.split_at(third);
        let (p2, p3) = rest.split_at(third);
        let (mut p2, mut p3) = (
            p2.iter().map(FlagDef::help_def),
            p3.iter().map(FlagDef::help_def),
        );
        let table = Table::new("  {:<}    {:<}    {:<}");
        let table = p1.iter().fold(table, |table, f1| {
            let (f2, f3) = (p2.next().unwrap_or_default(), p3.next().unwrap_or_default());
            table.with_row(row!(f1.help_def(), f2, f3))
        });
        help.push_str(&format!("{table}"));

        help
    }
}
