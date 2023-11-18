use std::{
    env,
    io::{BufRead as _, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Result;
use cargo_fixture::rpc::{PipeRequest, PipeResponse};
use log::{debug, info, trace};

use crate::{cli::Cli, utils::CommandExt};

pub struct FixtureProcess {
    cli: Cli,
    child: Child,
    msg_rx: mpsc::Receiver<PipeRequest>,
    child_stdin: ChildStdin,
}

impl FixtureProcess {
    pub fn spawn(cli: Cli) -> Result<Self> {
        let fixture_cmd = cli.fixture_cmd();
        debug!("running {}", fixture_cmd.display());
        let mut child = cli.fixture_cmd().spawn().unwrap(); // FIXME: err handling

        let msg_rx = Self::read_thread(child.stdout.take().unwrap());
        let child_stdin = child.stdin.take().unwrap();

        Ok(Self {
            cli,
            child,
            msg_rx,
            child_stdin,
        })
    }

    pub fn serve(&mut self) {
        let mut run = true;
        while run {
            // FIXME: EOF handling
            // FIXME: timeout - interruptible
            let resp = match self.msg_rx.recv().unwrap() {
                PipeRequest::SetEnv { name, value } => self.handle_set_env(name, value),
                PipeRequest::Ready => self.handle_ready(),
                PipeRequest::Finalize => {
                    info!("tearing down...");
                    run = false;
                    PipeResponse::Ok
                }
            };

            let mut resp = serde_json::to_string(&resp).unwrap();
            trace!("rpc response: {resp}");
            resp.push('\n');
            self.child_stdin.write_all(resp.as_bytes()).unwrap();
        }
    }

    fn handle_set_env(&self, name: String, value: String) -> PipeResponse {
        debug!("setting env var {name}={value}");
        env::set_var(name, value);
        PipeResponse::Ok
    }

    fn handle_ready(&self) -> PipeResponse {
        let mut test_cmd = self.cli.test_cmd();
        info!("running {}", test_cmd.display());
        let success = test_cmd
            .status()
            .map(|status| {
                debug!("test command: {status:?}");
                status.success()
            })
            // .map_err(Into::into)
            .unwrap();

        PipeResponse::TestsFinished { success }
    }

    fn read_thread(child_stdout: ChildStdout) -> mpsc::Receiver<PipeRequest> {
        let child_out = BufReader::new(child_stdout).lines();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for line in child_out {
                // FIXME: err handling:
                let line = line.unwrap();
                trace!("pipe: {}", line);
                let msg = serde_json::from_str(&line).unwrap();
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });

        rx
    }

    pub fn join(mut self) {
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                status.success(); // FIXME:
                return;
            }

            thread::sleep(Duration::from_millis(50));
        }
    }
}
