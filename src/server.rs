use std::{
    collections::HashMap,
    env, mem,
    process::Stdio,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::{bail, Context, Result};
use log::{debug, info, trace, warn};
use smol::Task;

use cargo_fixture::rpc_socket::{ConnectionType, Request, Response, RpcSocket};

use crate::{config::Config, utils::CommandExt as _};

mod server_socket;
use server_socket::ServerSocket;

type KvStore = Arc<RwLock<HashMap<String, serde_json::Value>>>;

pub struct Server {
    config: Arc<Config>,
    socket: ServerSocket,
    kv_store: KvStore,
    test_conns: Mutex<Vec<Task<()>>>,
}

impl Server {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let socket = ServerSocket::new(&config.socket_path)?;
        Ok(Self {
            config,
            socket,
            kv_store: KvStore::default(),
            test_conns: Default::default(),
        })
    }

    pub async fn accept_fixture(&self) -> Result<FixtureConnection> {
        let (socket, conn_type) = self
            .socket
            .accept()
            .await
            .context("Fixture connection error")?;
        if conn_type != ConnectionType::Fixture {
            bail!("Unexpected connection {conn_type:?}, expected fixture connection first");
        }

        Ok(FixtureConnection::new(
            socket,
            self.config.clone(),
            self.kv_store.clone(),
        ))
    }

    pub async fn accept_tests(self) -> Result<()> {
        loop {
            let conn = self.socket.accept().await?;
            self.handle_test_connection(conn).await?;
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
pub struct FixtureConnection {
    socket: RpcSocket,
    config: Arc<Config>,
    kv_store: KvStore,
    extra_test_args: Vec<String>,
    extra_harness_args: Vec<String>,
    replace_exec: Vec<String>,
}

impl FixtureConnection {
    fn new(socket: RpcSocket, config: Arc<Config>, kv_store: KvStore) -> Self {
        Self {
            socket,
            config,
            kv_store,
            extra_test_args: vec![],
            extra_harness_args: vec![],
            replace_exec: vec![],
        }
    }

    pub async fn run(mut self) -> Result<i32> {
        loop {
            let Some(req) = self.socket.recv().await? else {
                bail!("fixture program never called .ready(), tests not run");
            };

            let resp = match req {
                Request::SetEnv { name, value } => self.handle_set_env(name, value),
                Request::SetKeyValue { key, value } => self.handle_set_key_value(key, value),
                Request::GetKeyValue { key } => self.handle_get_key_value(key),
                Request::SetExtraTestArgs { args } => self.handle_set_extra_test_args(args),
                Request::SetExtraHarnessArgs { args } => self.handle_set_extra_harness_args(args),
                Request::SetExec { exec } => self.handle_set_exec(exec),

                Request::Ready => return self.run_tests().await,

                hello @ Request::Hello { .. } => bail!("Unexpected Hello message: {hello:?}"),
            };

            self.socket.send(resp).await?;
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

    fn handle_get_key_value(&mut self, key: String) -> Response {
        let value = self.kv_store.read().unwrap().get(&key).cloned();
        Response::KeyValue { key, value }
    }

    fn handle_set_extra_test_args(&mut self, args: Vec<String>) -> Response {
        debug!("setting extra cargo test args: {args:?}");
        self.extra_test_args = args;
        Response::Ok
    }

    fn handle_set_extra_harness_args(&mut self, args: Vec<String>) -> Response {
        debug!("setting extra test binary args: {args:?}");
        self.extra_harness_args = args;
        Response::Ok
    }

    fn handle_set_exec(&mut self, exec: Vec<String>) -> Response {
        if exec.is_empty() {
            debug!("resetting test command to default cargo test invocation");
        } else if !self.config.cli.exec.is_empty() {
            debug!("attempt to set test command to {exec:?} from fixture, but this is overriden by CLI flag -x");
        } else {
            debug!("setting test command to {exec:?}");
        }
        self.replace_exec = exec;
        Response::Ok
    }

    async fn run_tests(mut self) -> Result<i32> {
        trace!("KV storage: {:?}", *self.kv_store.read().unwrap());

        let extra_test_args = mem::take(&mut self.extra_test_args);
        let extra_harness_args = mem::take(&mut self.extra_harness_args);
        let replace_exec = mem::take(&mut self.replace_exec);
        let test_cmd = self
            .config
            .test_cmd(extra_test_args, extra_harness_args, replace_exec)?;
        info!("running {}", test_cmd.display());
        let status = test_cmd
            .into_smol(Stdio::inherit(), Stdio::inherit(), Stdio::inherit())
            .status()
            .await;
        debug!("test command: {status:?}");

        let success = status.as_ref().map(|s| s.success()).unwrap_or(false);
        let resp = Response::TestsFinished { success };
        self.socket.send(resp).await?;

        status
            .map(|s| s.code().unwrap_or(1))
            .context("test command error")
    }
}

/// Handles connection from individual tests.
struct TestConnection {
    socket: RpcSocket,
    kv_store: KvStore,
}

impl TestConnection {
    fn new(socket: RpcSocket, kv_store: KvStore) -> Self {
        Self { socket, kv_store }
    }

    async fn run(mut self) {
        if let Err(err) = self.run_inner().await {
            warn!("Test connection error: {err}");
        }
    }

    async fn run_inner(&mut self) -> Result<()> {
        loop {
            let Some(req) = self.socket.recv().await? else {
                return Ok(());
            };
            let resp = match req {
                Request::GetKeyValue { key } => self.handle_get_key_value(key),
                other => bail!("Unexpected message: {other:?}"),
            };
            self.socket.send(resp).await?;
        }
    }

    fn handle_get_key_value(&mut self, key: String) -> Response {
        let value = self.kv_store.read().unwrap().get(&key).cloned();
        Response::KeyValue { key, value }
    }
}
