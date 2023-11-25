use std::{env, path::PathBuf, process::ExitStatus, future::Future};

use anyhow::{Context, Result, Error, bail};
use async_ctrlc::CtrlC;
use cargo_fixture::rpc::{Request, Response};
use futures_util::{TryFutureExt as _, pin_mut, select, FutureExt};
use log::{debug, info, Level};
use smol::process::{Child, Command};

use crate::{
    config::Config,
    utils::{CommandExt as _, RmGuard},
};

use self::socket::{Connection, Socket};

mod socket;

pub struct FixtureProcess {
    config: Config,
    child: Child,
    socket: Connection,
    ctrlc: CtrlC,
    data_tmp_files: Vec<RmGuard<PathBuf>>,
}

enum FixtureOp<T> {
    Ok(T),
    Process(ExitStatus),
}

async fn fixture_op<T, F>(ft: F, child: &mut Child, ctrlc: &mut CtrlC) -> Result<FixtureOp<T>> where F: Future<Output = Result<T>> {
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
            .context("Error launching fixture process")?;

        let socket = match fixture_op(socket.accept(), &mut child, &mut ctrlc).await? {
            FixtureOp::Ok(socket) => socket,
            FixtureOp::Process(status) => {
                match status.code() {
                    Some(0) => bail!("Fixture process didn't connect to cargo fixture"),
                    Some(c) => bail!("Fixture process failed, exit code: {c}"),
                    None => bail!("Fixture process killed by a signal"),
                }
            },
        };

        Ok(Self {
            config,
            child,
            socket,
            ctrlc,
            data_tmp_files: vec![],
        })
    }

    pub async fn serve(&mut self) -> Result<()> {
        let mut run = true;
        while run {
            let request = match fixture_op(self.socket.recv(), &mut self.child, &mut self.ctrlc).await? {
                FixtureOp::Ok(Some(req)) => req,
                FixtureOp::Ok(None) => return Ok(()),
                FixtureOp::Process(_) => todo!(),
            };

            let resp = match request {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::EnqueueData { key, path } => self.handle_enqueue_data(key, path),
                Request::Ready => {
                    run = false;
                    self.handle_ready()
                },
            };

            self.socket.send(resp).await.expect("FIXME:");  // TODO: the connection may no longer be writeable
        }

        Ok(())
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

    fn handle_ready(&self) -> Response {
        let mut test_cmd = self.config.test_cmd();
        info!("running {}", test_cmd.display());
        let success = test_cmd
            .status()
            .map(|status| {
                debug!("test command: {status:?}");
                status.success()
            })
            .map_err(|err| {
                debug!("test command error: {err}");
                err
            })
            .unwrap_or(false);

        Response::TestsFinished { success }
    }

    pub async fn join(mut self) {
        self.child.status().await.expect("FIXME:");
    }
}
