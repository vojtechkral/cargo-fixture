use std::{
    ffi::OsStr,
    fmt, fs,
    path::Path,
    process::{Command, ExitStatus},
};

use anyhow::{bail, Context, Result};
use log::{log, warn, Level};

pub trait CommandExt {
    fn display<'a>(&'a self) -> CommandPrint<'a>;
}

impl CommandExt for Command {
    fn display<'a>(&'a self) -> CommandPrint<'a> {
        CommandPrint(self)
    }
}

pub struct CommandPrint<'a>(&'a Command);

impl<'a> fmt::Display for CommandPrint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get_program().to_string_lossy())?;
        for arg in self.0.get_args() {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct RmGuard<P: AsRef<Path>> {
    path: P,
    log_level: Level,
}

impl<P: AsRef<Path>> RmGuard<P> {
    pub fn new(path: P, log_level: Level) -> Self {
        Self { path, log_level }
    }
}

impl<P> Drop for RmGuard<P>
where
    P: AsRef<Path>,
{
    fn drop(&mut self) {
        let p = self.path.as_ref();
        log!(self.log_level, "removing {}", p.display());
        if let Err(err) = fs::remove_file(p) {
            warn!("could not remove file `{}`: {}", p.display(), err);
        }
    }
}

pub trait ExitStatusExt {
    fn as_result(&self) -> Result<()>;
    fn fixture_early_exit<T>(&self) -> Result<T>;
}

impl ExitStatusExt for ExitStatus {
    fn as_result(&self) -> Result<()> {
        match self.code() {
            Some(0) => Ok(()),
            Some(c) => bail!("Exit code: {c}"),
            None => bail!("Process killed by a signal"),
        }
    }

    fn fixture_early_exit<T>(&self) -> Result<T> {
        self.as_result().context("Fixture failed")?;
        bail!("Fixture didn't connect to cargo fixture")
    }
}

pub trait OsStrExt {
    fn starts_with(&self, c: char) -> bool;
}

impl<'a> OsStrExt for &'a OsStr {
    fn starts_with(&self, c: char) -> bool {
        self.to_string_lossy().chars().next() == Some(c)
    }
}
