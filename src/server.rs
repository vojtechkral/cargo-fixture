use std::{
    collections::HashMap,
    env, mem,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::{bail, Result};

use futures_util::{pin_mut, select, FutureExt};
use log::{debug, info, warn};

use cargo_fixture::rpc_socket::{ConnectionType, Request, Response, RpcSocket};
use smol::Task;

use crate::{config::Config, utils::CommandExt as _, CtrlC};

mod server_socket;
use server_socket::ServerSocket;

type KvStore = Arc<RwLock<HashMap<String, serde_json::Value>>>;

pub struct Server {
    config: Arc<Config>,
    socket: ServerSocket,
    ctrlc: CtrlC,
    kv_store: KvStore,
    test_conns: Mutex<Vec<Task<()>>>,
}

impl Server {
    pub fn new(config: Arc<Config>, ctrlc: CtrlC) -> Result<Self> {
        let socket = ServerSocket::new(&config.socket_path)?;
        Ok(Self {
            config,
            socket,
            ctrlc,
            kv_store: KvStore::default(),
            test_conns: Default::default(),
        })
    }

    pub async fn run(self) -> Result<i32> {
        let (socket, conn_type) = self.socket.accept().await?;
        if conn_type != ConnectionType::Fixture {
            bail!("Unexpected connection {conn_type:?}, expected fixture connection first");
        }

        let fixture = FixtureConnection::new(socket, self.config.clone(), self.kv_store.clone());
        let mut fixture = smol::spawn(fixture.run()).fuse();

        loop {
            let conn = self.socket.accept().fuse();
            pin_mut!(conn);
            select! {
                res = conn => self.handle_test_connection(res?).await?,
                res = fixture => return res,
            }
        }
    }

    pub async fn handle_test_connection(
        &self,
        (socket, conn_type): (RpcSocket, ConnectionType),
    ) -> Result<()> {
        let serial = match conn_type {
            ConnectionType::Client => false,
            ConnectionType::ClientSerial => true,
            ConnectionType::Fixture => {
                bail!("Unexpected connection {conn_type:?}, expected test connection")
            }
        };

        if !serial {
            let test = TestConnection::new(socket, self.kv_store.clone());
            let task = smol::spawn(test.run());
            self.test_conns.lock().unwrap().push(task);
        } else {
            // 1. wait for all outstanding test tasks to finish
            let test_conns = mem::take(&mut *self.test_conns.lock().unwrap());
            for task in test_conns {
                task.await;
            }

            // 2. run the serial test and wait for it to finish
            TestConnection::new(socket, self.kv_store.clone())
                .run()
                .await;
        }

        Ok(())
    }
}

/// Handles connection from the fixture process, spawns `cargo test` as part of this.
struct FixtureConnection {
    socket: RpcSocket,
    config: Arc<Config>,
    kv_store: KvStore,
    extra_test_args: Vec<String>,
    extra_harness_args: Vec<String>,
    return_status: Option<i32>,
}

impl FixtureConnection {
    fn new(socket: RpcSocket, config: Arc<Config>, kv_store: KvStore) -> Self {
        Self {
            socket,
            config,
            kv_store,
            extra_test_args: vec![],
            extra_harness_args: vec![],
            return_status: None,
        }
    }

    async fn run(mut self) -> Result<i32> {
        loop {
            let req = self.socket.recv().await?;
            let resp = match req {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::SetKeyValue { key, value } => self.handle_set_key_value(key, value),
                Request::SetExtraTestArgs { args } => self.handle_set_extra_test_args(args),
                Request::SetExtraHarnessArgs { args } => self.handle_set_extra_harness_args(args),
                Request::Ready => self.handle_ready(),
                req @ Request::Hello { .. } => panic!("Unexpected Hello message {req:?}"),
            };
            self.socket.send(resp).await?;

            if let Some(status) = self.return_status {
                return Ok(status);
            }
        }
    }

    fn handle_set_env(&self, name: String, value: String) -> Response {
        debug!("setting env var {name}={value}");
        env::set_var(name, value);
        Response::Ok
    }

    fn handle_set_key_value(&mut self, key: String, value: serde_json::Value) -> Response {
        debug!("storing KV data for key `{key}`");
        self.kv_store.write().unwrap().insert(key, value);
        Response::Ok
    }

    fn handle_set_extra_test_args(&mut self, args: Vec<String>) -> Response {
        debug!("setting extra cargo test args: {args:?}");
        self.extra_test_args = args;
        Response::Ok
    }

    fn handle_set_extra_harness_args(&mut self, args: Vec<String>) -> Response {
        debug!("setting test binary args: {args:?}");
        self.extra_harness_args = args;
        Response::Ok
    }

    fn handle_ready(&mut self) -> Response {
        let success = self.run_tests();
        Response::TestsFinished { success }
    }

    fn run_tests(&mut self) -> bool {
        let extra_test_args = mem::take(&mut self.extra_test_args);
        let extra_harness_args = mem::take(&mut self.extra_harness_args);
        let mut test_cmd = self.config.test_cmd(extra_test_args, extra_harness_args);
        info!("running {}", test_cmd.display());
        test_cmd
            .status()
            .map(|status| {
                debug!("test command: {status:?}");
                self.return_status = status.code().or(Some(1));
                status.success()
            })
            .map_err(|err| {
                warn!("test command error: {err}");
                err
            })
            .unwrap_or(false)
    }
}

/// Handles connection from individual tests.
struct TestConnection {
    socket: RpcSocket,
    // config: Arc<Config>,  // TODO: needed?
    kv_store: KvStore,
}

impl TestConnection {
    fn new(socket: RpcSocket, kv_store: KvStore) -> Self {
        Self { socket, kv_store }
    }

    async fn run(self) {
        todo!()
    }
}
