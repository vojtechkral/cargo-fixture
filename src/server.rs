use std::env;

use anyhow::{bail, Context, Result};
use async_ctrlc::CtrlC;

use log::debug;
#[cfg(unix)]
use smol::net::unix::UnixStream;
// Windows  TODO:
#[cfg(windows)]
use uds_windows::UnixListener;

use cargo_fixture::rpc_socket::{ConnectionType, Request, Response, RpcSocket};

use crate::config::Config;

mod socket;
use socket::ServerSocket;

pub struct Server {
    config: Config,
    socket: ServerSocket,
    ctrlc: CtrlC,
}

impl Server {
    pub fn new(config: Config) -> Result<Self> {
        let ctrlc = CtrlC::new().context("Failed to create SIGINT handler")?;
        let socket = ServerSocket::new(&config.socket_path)?;
        Ok(Self {
            config,
            socket,
            ctrlc,
        })
    }

    pub async fn run(self) -> Result<i32> {
        let (socket, conn_type) = self.socket.accept().await?;
        if conn_type != ConnectionType::Fixture {
            bail!("Unexpected connection {conn_type:?}, expected fixture connection first");
        }

        let fixture = smol::spawn(FixtureConnection::new(socket).run());

        // TODO: handle test connections

        fixture.await
    }
}

/// Handles connection from the fixture process, spawns `cargo test` as part of this.
struct FixtureConnection {
    socket: RpcSocket,
}

impl FixtureConnection {
    fn new(socket: RpcSocket) -> Self {
        Self { socket }
    }

    async fn run(mut self) -> Result<i32> {
        loop {
            let req = self.socket.recv().await?;
            let resp = match req {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::EnqueueData { key, path } => todo!(), // FIXME: replace
                Request::SetAdditionalArgs {
                    to_cargo_test,
                    to_harness,
                } => todo!(),
                Request::Ready => todo!(),

                req @ Request::Hello { .. } => panic!("Unexpected message {req:?}"),
            };
            self.socket.send(resp).await?;
        }
    }

    fn handle_set_env(&self, name: String, value: String) -> Response {
        debug!("setting env var {name}={value}");
        env::set_var(name, value);
        Response::Ok
    }

    // fn handle_enqueue_data(&mut self, key: String, path: PathBuf) -> Response {
    //     debug!("fixture data set, key: `{key}` -> {}", path.display());
    //     self.data_tmp_files.push(RmGuard::new(path, Level::Debug));
    //     Response::Ok
    // }

    // fn handle_set_add_args(
    //     &mut self,
    //     to_cargo_test: Option<Vec<String>>,
    //     to_harness: Option<Vec<String>>,
    // ) -> Response {
    //     debug!("set additional args to cargo test: {to_cargo_test:?}, to harness: {to_harness:?}");
    //     to_cargo_test.map(|args| self.args_to_cargo_test = args);
    //     to_harness.map(|args| self.args_to_harness = args);
    //     Response::Ok
    // }

    // fn handle_ready(&mut self) -> Response {
    //     let success = self.run_tests();
    //     let resp = Response::TestsFinished { success };
    //     info!("tearing down...");
    //     resp
    // }

    // fn run_tests(&mut self) -> bool {
    //     let add_args_to_cargo_test = mem::take(&mut self.args_to_cargo_test);
    //     let add_args_to_harness = mem::take(&mut self.args_to_harness);
    //     let mut test_cmd = self
    //         .config
    //         .test_cmd(add_args_to_cargo_test, add_args_to_harness);
    //     info!("running {}", test_cmd.display());
    //     test_cmd
    //         .status()
    //         .map(|status| {
    //             debug!("test command: {status:?}");
    //             self.test_status = status.code().unwrap_or(1);
    //             status.success()
    //         })
    //         .map_err(|err| {
    //             warn!("test command error: {err}");
    //             err
    //         })
    //         .unwrap_or(false)
    // }
}
