use std::{
    env,
    io::{BufRead as _, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration, fmt,
};

use anyhow::Result;
use cargo_fixture::rpc::{PipeRequest, PipeResponse};
use log::{info, trace};

#[derive(Clone, Debug)]
pub struct CmdSpec {
    program: String,
    args: Vec<String>,
}

impl CmdSpec {
    pub fn new(program: String, args: Vec<String>) -> Self { Self { program, args } }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
            cmd.args(&self.args[..]);
            cmd
    }
}

impl fmt::Display for CmdSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args[..] {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

pub struct FixtureProcess {
    test_cmd: CmdSpec,
    child: Child,
    msg_rx: mpsc::Receiver<PipeRequest>,
    child_stdin: ChildStdin,
}

impl FixtureProcess {
    pub fn spawn(fixture_cmd: CmdSpec,test_cmd: CmdSpec) -> Result<Self> {
        let mut child = fixture_cmd.command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(); // FIXME: err handling

        let msg_rx = Self::read_thread(child.stdout.take().unwrap());
        let child_stdin = child.stdin.take().unwrap();

        Ok(Self {
            test_cmd,
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
                PipeRequest::RunTests => self.handle_run_tests(),
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
        info!("setting env var {name}={value}"); // TODO: log level?
        env::set_var(name, value);
        PipeResponse::Ok
    }

    fn handle_run_tests(&self) -> PipeResponse {
        info!("running {}", self.test_cmd);
        let success = self.test_cmd.command()
            .status()
            .map(|status| status.success())
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
