use std::{env, fs, mem, path::PathBuf, process::Child, thread, time::Duration};

use anyhow::Result;
use cargo_fixture::{
    rpc::{PipeRequest, PipeResponse},
    socket::Socket,
};
use log::{debug, info, warn};

use crate::{config::Config, utils::CommandExt};

pub struct FixtureProcess {
    config: Config,
    child: Child,
    socket: Socket,
    data_tmp_files: Vec<PathBuf>,
}

impl FixtureProcess {
    pub fn spawn(config: Config) -> Result<Self> {
        let fixture_cmd = config.fixture_cmd();
        debug!("running {}", fixture_cmd.display());
        let child = config.fixture_cmd().spawn().unwrap(); // FIXME: err handling

        // FIXME: child may never connect (eg. cargo error) - move to thread
        let socket = Socket::accept(&config.socket_path);

        Ok(Self {
            config,
            child,
            socket,
            data_tmp_files: vec![],
        })
    }

    pub fn serve(&mut self) {
        let mut run = true;
        while run {
            // FIXME: EOF handling
            // FIXME: timeout - interruptible

            let request: PipeRequest = self.socket.recv();
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

            self.socket.send(resp);
            // let mut resp = serde_json::to_string(&resp).unwrap();
            // trace!("rpc response: {resp}");
            // resp.push('\n');
            // self.child_stdin.write_all(resp.as_bytes()).unwrap();
        }
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

    pub fn join(mut self) {
        // create a guard that will clean up fixture data files
        let _data_files_cleanup = RmDataFilesGuard::new(&mut self.data_tmp_files);

        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                status.success(); // FIXME:
                return;
            }

            thread::sleep(Duration::from_millis(50));
        }
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
