use std::{env, future::Future, mem, path::PathBuf, process::ExitStatus};

use anyhow::{bail, Context, Error, Result};
use async_ctrlc::CtrlC;
use cargo_fixture::rpc::{Request, Response, WithVersion};
use futures_util::{pin_mut, select, FutureExt, TryFutureExt as _};
use log::{debug, info, warn, Level};
use smol::process::{Child, Command};

use crate::{
    config::Config,
    utils::{CommandExt as _, ExitStatusExt, RmGuard},
};

use self::socket::{Connection, Socket};

pub mod socket; // TODO: move

pub struct FixtureProcess {
    config: Config,
    child: Child,
    socket: Connection,
    ctrlc: CtrlC,
    version_checked: bool,
    args_to_cargo_test: Vec<String>,
    args_to_harness: Vec<String>,
    test_status: i32,
    data_tmp_files: Vec<RmGuard<PathBuf>>,
}

enum FixtureOp<T> {
    Ok(T),
    Process(ExitStatus),
}

async fn fixture_op<T, F>(ft: F, child: &mut Child, ctrlc: &mut CtrlC) -> Result<FixtureOp<T>>
where
    F: Future<Output = Result<T>>,
{
    let ft = ft.fuse();
    pin_mut!(ft);
    let status = child.status().map_err(Error::from);
    pin_mut!(status);
    let mut ctrlc = ctrlc.fuse();

    select! {
        res = ft => res.map(FixtureOp::Ok),
        res = status => res.map(FixtureOp::Process),
        _ = ctrlc => bail!("Interrupted."),
    }
}

impl FixtureProcess {
    pub async fn spawn(config: Config, mut ctrlc: CtrlC) -> Result<Self> {
        let socket = Socket::new(&config.socket_path)?;

        let fixture_cmd = config.fixture_cmd();
        debug!("running {}", fixture_cmd.display());
        let mut child = Command::from(fixture_cmd)
            .spawn()
            .context("Error launching fixture")?;

        let socket = match fixture_op(socket.accept(), &mut child, &mut ctrlc).await? {
            FixtureOp::Ok(socket) => socket,
            FixtureOp::Process(status) => return status.fixture_early_exit(),
        };

        Ok(Self {
            config,
            child,
            socket,
            ctrlc,
            version_checked: false,
            args_to_cargo_test: vec![],
            args_to_harness: vec![],
            test_status: 0,
            data_tmp_files: vec![],
        })
    }

    pub async fn serve(&mut self) -> Result<i32> {
        let mut run = true;
        while run {
            let Some(resp) = self.recv().await? else {
                self.run_tests();
                return Ok(self.test_status);
            };
            let resp = match resp {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::EnqueueData { key, path } => self.handle_enqueue_data(key, path),
                Request::SetAdditionalArgs {
                    to_cargo_test,
                    to_harness,
                } => self.handle_set_add_args(to_cargo_test, to_harness),
                Request::Ready => {
                    run = false;
                    self.handle_ready()
                }
            };

            self.socket
                .send(resp)
                .await
                .context("Error sending reply to fixture process")?;
        }

        Ok(self.test_status)
    }

    async fn recv(&mut self) -> Result<Option<Request>> {
        let WithVersion {
            ver: theirs,
            request,
        } = match fixture_op(self.socket.recv(), &mut self.child, &mut self.ctrlc).await? {
            FixtureOp::Ok(Some(req)) => req,
            FixtureOp::Ok(None) => return Ok(None),
            FixtureOp::Process(status) => return status.fixture_early_exit(),
        };

        if !self.version_checked {
            let ours = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
            if ours != theirs {
                bail!("This cargo-fixture binary version ({ours}.x.y) is not compatible with the library linked by test code ({theirs}.x.y)");
            }
            self.version_checked = true;
        }

        Ok(Some(request))
    }

    fn handle_set_env(&self, name: String, value: String) -> Response {
        debug!("setting env var {name}={value}");
        env::set_var(name, value);
        Response::Ok
    }

    fn handle_enqueue_data(&mut self, key: String, path: PathBuf) -> Response {
        debug!("fixture data set, key: `{key}` -> {}", path.display());
        self.data_tmp_files.push(RmGuard::new(path, Level::Debug));
        Response::Ok
    }

    fn handle_set_add_args(
        &mut self,
        to_cargo_test: Option<Vec<String>>,
        to_harness: Option<Vec<String>>,
    ) -> Response {
        debug!("set additional args to cargo test: {to_cargo_test:?}, to harness: {to_harness:?}");
        to_cargo_test.map(|args| self.args_to_cargo_test = args);
        to_harness.map(|args| self.args_to_harness = args);
        Response::Ok
    }

    fn handle_ready(&mut self) -> Response {
        let success = self.run_tests();
        let resp = Response::TestsFinished { success };
        info!("tearing down...");
        resp
    }

    fn run_tests(&mut self) -> bool {
        let add_args_to_cargo_test = mem::take(&mut self.args_to_cargo_test);
        let add_args_to_harness = mem::take(&mut self.args_to_harness);
        let mut test_cmd = self
            .config
            .test_cmd(add_args_to_cargo_test, add_args_to_harness);
        info!("running {}", test_cmd.display());
        test_cmd
            .status()
            .map(|status| {
                debug!("test command: {status:?}");
                self.test_status = status.code().unwrap_or(1);
                status.success()
            })
            .map_err(|err| {
                warn!("test command error: {err}");
                err
            })
            .unwrap_or(false)
    }

    pub async fn join(mut self) -> Result<ExitStatus> {
        self.child
            .status()
            .await
            .context("I/O error while joining the fixture process")
    }
}
