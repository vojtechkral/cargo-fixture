use std::{
    ascii,
    ffi::OsStr,
    fmt, fs,
    path::Path,
    pin::Pin,
    process::{Command, ExitStatus, Stdio},
    task::{self, Poll},
    time::{Duration, Instant},
};

use anyhow::{bail, Context as _, Ok, Result};
use futures_util::{future::FusedFuture, ready, Future, StreamExt};
use log::{error, log, warn, Level};
use os_str_bytes::RawOsStr;
use smol::{channel, process::Command as SmolCommand};

pub trait ResultExt {
    fn log_error(self);
}

impl<T> ResultExt for Result<T> {
    fn log_error(self) {
        if let Err(err) = self {
            error!("{err:?}")
        }
    }
}

pub trait CommandExt {
    fn display(&self) -> CommandPrint<'_>;

    /// `SmolCommand::from()` won't take stdio config from `Command` (it can't),
    /// this function performs the conversion and sets up stdio.
    fn into_smol(self, stdin: Stdio, stdout: Stdio, stderr: Stdio) -> SmolCommand;
}

impl CommandExt for Command {
    fn display(&self) -> CommandPrint<'_> {
        CommandPrint(self)
    }

    fn into_smol(self, stdin: Stdio, stdout: Stdio, stderr: Stdio) -> SmolCommand {
        let mut cmd = SmolCommand::from(self);
        cmd.stdin(stdin).stdout(stdout).stderr(stderr);
        cmd
    }
}

pub struct CommandPrint<'a>(&'a Command);

impl<'a> fmt::Display for CommandPrint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get_program().to_string_lossy())?;
        for arg in self.0.get_args() {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        fmt::Result::Ok(())
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
    fn as_result(&self, context: &str) -> Result<()>;
}

impl ExitStatusExt for ExitStatus {
    fn as_result(&self, context: &str) -> Result<()> {
        match self.code() {
            Some(0) => Ok(()),
            Some(c) => bail!("{context}: exit code: {c}"),
            None => bail!("{context}: killed by a signal"),
        }
    }
}

pub trait StringExt {
    fn push_strs(&mut self, strs: &[&str]);
}

impl StringExt for String {
    fn push_strs(&mut self, strs: &[&str]) {
        strs.iter().for_each(|s| self.push_str(s));
    }
}

pub trait OsStrExt {
    fn starts_with(&self, c: char) -> bool;
    fn to_escaped(&self) -> String;
}

impl<T> OsStrExt for T
where
    T: AsRef<OsStr>,
{
    fn starts_with(&self, c: char) -> bool {
        self.as_ref().to_string_lossy().starts_with(c)
    }

    fn to_escaped(&self) -> String {
        let os = RawOsStr::new(self);
        let bytes = os.to_raw_bytes();
        bytes
            .iter()
            .flat_map(|&b| ascii::escape_default(b))
            .map(|b| char::from_u32(b as _).unwrap())
            .collect::<String>()
    }
}

/// Return a `Future` that resolves when Ctrl+C is pressed twice in a quick succession.
pub fn ctrlc_2x() -> Result<CtrlC<2>> {
    CtrlC::new()
}

pub struct CtrlC<const N: usize> {
    rx: channel::Receiver<Instant>,
    num_successions: usize,
    last_timestamp: Instant,
}

impl<const N: usize> CtrlC<N> {
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel::bounded(10);

        ctrlc::set_handler(move || {
            let _ = tx.try_send(Instant::now());
        })
        .context("Failed to set up SIGINT handler")?;

        Ok(Self {
            rx,
            num_successions: 1,
            last_timestamp: Instant::now().checked_sub(Self::INTERVAL).unwrap(),
        })
    }

    const INTERVAL: Duration = Duration::from_millis(400);
}

impl<const N: usize> Future for CtrlC<N> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.as_mut();

        loop {
            if this.num_successions >= N {
                return Poll::Ready(());
            }

            if let Some(timestamp) = ready!(this.rx.poll_next_unpin(cx)) {
                if timestamp.duration_since(this.last_timestamp) <= Self::INTERVAL {
                    this.num_successions += 1;
                }
                this.last_timestamp = timestamp;
            } else {
                // Channel closed, never resolve
                return Poll::Pending;
            }
        }
    }
}

impl<const N: usize> FusedFuture for CtrlC<N> {
    fn is_terminated(&self) -> bool {
        self.num_successions >= N
    }
}
