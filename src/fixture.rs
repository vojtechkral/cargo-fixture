use std::{
    env,
    io::{BufRead as _, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Result;
use cargo_fixture::rpc::{PipeRequest, PipeResponse};
use log::{info, trace};

pub struct FixtureProcess {
    child: Child,
    msg_rx: mpsc::Receiver<PipeRequest>,
    child_stdin: ChildStdin,
}

impl FixtureProcess {
    pub fn spawn(cargo: &str) -> Result<Self> {
        // FIXME: Customizable? ie. workspace package etc.
        //        use cargo metadata? https://doc.rust-lang.org/cargo/reference/manifest.html#the-metadata-table
        let mut child = Command::new(cargo)
            .args(["test", "--test", "fixture"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(); // FIXME: err handling

        let msg_rx = Self::read_thread(child.stdout.take().unwrap());
        let child_stdin = child.stdin.take().unwrap();

        Ok(Self {
            child,
            msg_rx,
            child_stdin,
        })
    }

    pub fn serve(&mut self) {
        loop {
            // FIXME: EOF handling
            // FIXME: timeout - interruptible
            let resp = match dbg!(self.msg_rx.recv().unwrap()) {
                PipeRequest::SetEnv { name, value } => self.handle_set_env(name, value),
                PipeRequest::RunTests => self.handle_run_tests(),
                PipeRequest::Finalize => {
                    info!("tearing down...");
                    return;
                }
            };

            let mut msg = serde_json::to_string(&resp).unwrap();
            msg.push('\n');
            self.child_stdin.write_all(msg.as_bytes()).unwrap();
        }
    }

    fn handle_set_env(&self, name: String, value: String) -> PipeResponse {
        info!("setting env var {name}={value}"); // TODO: log level?
        env::set_var(name, value);
        PipeResponse::Ok
    }

    fn handle_run_tests(&self) -> PipeResponse {
        todo!()
    }

    // // FIXME: rm
    // pub fn teardown(&mut self, tests_success: bool) {
    //     // FIXME: err handling
    //     self.write_msg(PipeMsg::TestsFinished {
    //         success: tests_success,
    //     }).unwrap();
    //     loop {
    //         if let Some(status) = self.child.try_wait().unwrap() {
    //             status.success(); // FIXME:
    //             return
    //         }

    //         thread::sleep(Duration::from_millis(50));
    //     }
    // }

    fn read_thread(child_stdout: ChildStdout) -> mpsc::Receiver<PipeRequest> {
        let child_out = BufReader::new(child_stdout).lines();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for line in child_out {
                // FIXME: err handling:
                let line = line.unwrap();
                dbg!("line"); // FIXME: ??
                let msg = serde_json::from_str(&line).unwrap();
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });

        rx
    }

    // fn write_msg(&mut self, msg: PipeMsg) -> Result<()> {
    //     // FIXME: err handling
    //     let mut msg = serde_json::to_string(&msg).unwrap();
    //     msg.push('\n');
    //     self.child_stdin.write_all(msg.as_bytes()).unwrap();
    //     Ok(())
    // }

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
