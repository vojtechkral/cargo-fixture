use std::{env, future::Future, path::PathBuf, process::ExitStatus};

use anyhow::{anyhow, bail, Context, Error, Result};
use async_ctrlc::CtrlC;
use cargo_fixture::rpc::{Request, Response, WithVersion};
use futures_util::{pin_mut, select, FutureExt, TryFutureExt as _};
use log::{debug, info, warn, Level};
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
    version_checked: bool,
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
            .context("Error launching fixture process")?;

        let socket = match fixture_op(socket.accept(), &mut child, &mut ctrlc).await? {
            FixtureOp::Ok(socket) => socket,
            FixtureOp::Process(status) => return Err(Self::fixture_early_exit(status)),
        };

        Ok(Self {
            config,
            child,
            socket,
            ctrlc,
            version_checked: false,
            data_tmp_files: vec![],
        })
    }

    pub async fn serve(&mut self) -> Result<()> {
        let mut run = true;
        while run {
            let Some(resp) = self.recv().await? else {
                self.run_tests();
                return Ok(());
            };
            let resp = match resp {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::EnqueueData { key, path } => self.handle_enqueue_data(key, path),
                Request::Ready => {
                    run = false;
                    self.handle_ready()
                }
                Request::Version { .. } => todo!(), // FIXME: error
            };

            self.socket.send(resp).await.expect("FIXME:"); // TODO: the connection may no longer be writeable
        }

        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<Request>> {
        let WithVersion {
            ver: theirs,
            request,
        } = match fixture_op(self.socket.recv(), &mut self.child, &mut self.ctrlc).await? {
            FixtureOp::Ok(Some(req)) => req,
            FixtureOp::Ok(None) => return Ok(None),
            FixtureOp::Process(status) => return Err(Self::fixture_early_exit(status)),
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

    fn fixture_early_exit(status: ExitStatus) -> Error {
        match status.code() {
            Some(0) => anyhow!("Fixture process didn't connect to cargo fixture"),
            Some(c) => anyhow!("Fixture process failed, exit code: {c}"),
            None => anyhow!("Fixture process killed by a signal"),
        }
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
        let success = self.run_tests();
        Response::TestsFinished { success }
    }

    fn run_tests(&self) -> bool {
        // TODO: propagate error status as exit status

        let mut test_cmd = self.config.test_cmd();
        info!("running {}", test_cmd.display());
        test_cmd
            .status()
            .map(|status| {
                debug!("test command: {status:?}");
                status.success()
            })
            .map_err(|err| {
                warn!("test command error: {err}");
                err
            })
            .unwrap_or(false)
    }

    pub async fn join(mut self) {
        self.child.status().await.expect("FIXME:");
    }
}
