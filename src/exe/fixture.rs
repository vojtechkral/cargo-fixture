use std::{env, fs, mem, path::PathBuf, process::ExitStatus};

use anyhow::{Context, Result};
use cargo_fixture::rpc::{PipeRequest, PipeResponse};
use log::{debug, info, warn};
use smol::{
    future::FutureExt as _,
    process::{Child, Command},
};

use crate::{
    config::Config,
    utils::{CommandExt as _, FutureExt},
};

use self::socket::{Connection, Socket};

mod socket;

pub struct FixtureProcess {
    config: Config,
    child: Child,
    socket: Connection,
    data_tmp_files: Vec<PathBuf>,
}

enum Recv {
    Socket(PipeRequest),
    Process(ExitStatus),
}

impl FixtureProcess {
    pub async fn spawn(config: Config) -> Result<Self> {
        let socket = Socket::new(&config.socket_path)?;

        let fixture_cmd = config.fixture_cmd();
        debug!("running {}", fixture_cmd.display());
        let child = Command::from(fixture_cmd)
            .spawn()
            .context("Error launching fixture process")?;

        // FIXME: race, reap process
        let socket = socket.accept().await?;

        Ok(Self {
            config,
            child,
            socket,
            data_tmp_files: vec![],
        })
    }

    pub async fn serve(&mut self) -> Result<()> {
        let mut run = true;
        while run {
            // FIXME: EOF handling

            let request = match self.child_recv().await? {
                Recv::Socket(req) => req,
                Recv::Process(_) => todo!(),
            };

            let resp = match request {
                PipeRequest::SetEnv { name, value } => self.handle_set_env(name, value),
                PipeRequest::EnqueueData { key, path } => self.handle_enqueue_data(key, path),
                PipeRequest::Ready => self.handle_ready(),
                PipeRequest::Finalize => {
                    info!("tearing down...");
                    run = false;
                    PipeResponse::Ok
                }
            };

            self.socket.send(resp).await.expect("FIXME:");
        }

        Ok(())
    }

    async fn child_recv(&mut self) -> Result<Recv> {
        self.socket
            .recv::<PipeRequest>()
            .map(|res| res.map(Recv::Socket))
            .or(self
                .child
                .status()
                .map(|res| res.context("FIXME:").map(Recv::Process)))
            .await
    }

    fn handle_set_env(&self, name: String, value: String) -> PipeResponse {
        debug!("setting env var {name}={value}");
        env::set_var(name, value);
        PipeResponse::Ok
    }

    fn handle_enqueue_data(&mut self, key: String, path: PathBuf) -> PipeResponse {
        debug!("fixture data set, key: `{key}` -> {}", path.display());
        self.data_tmp_files.push(path);
        PipeResponse::Ok
    }

    fn handle_ready(&self) -> PipeResponse {
        let mut test_cmd = self.config.test_cmd();
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

    pub async fn join(mut self) {
        // create a guard that will clean up fixture data files
        // FIXME: use RmGuard
        let _data_files_cleanup = RmDataFilesGuard::new(&mut self.data_tmp_files);
        self.child.status().await.expect("FIXME:");
    }
}

#[derive(Debug)]
struct RmDataFilesGuard(Vec<PathBuf>);

impl RmDataFilesGuard {
    fn new(from: &mut Vec<PathBuf>) -> Self {
        Self(mem::take(from))
    }
}

impl Drop for RmDataFilesGuard {
    fn drop(&mut self) {
        for path in self.0.iter() {
            debug!("removing {}", path.display());
            if let Err(err) = fs::remove_file(&path) {
                warn!("could not remove file `{}`: {}", path.display(), err);
            }
        }
    }
}
