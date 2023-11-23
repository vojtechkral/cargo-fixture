use std::{fmt, process::Command, path::Path, fs};

use log::trace;

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
pub(crate) struct RmGuard<P>(pub(crate) P) where P: AsRef<Path>;

impl<P> Drop for RmGuard<P> where P: AsRef<Path> {
    fn drop(&mut self) {
        let p = self.0.as_ref();
        trace!("removing {}", p.display());
        let _ = fs::remove_file(p);
    }
}
