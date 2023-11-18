use std::{fmt, process::Command};

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
