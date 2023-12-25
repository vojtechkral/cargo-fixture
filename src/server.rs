use std::{
    collections::HashMap,
    env, mem,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use futures_util::{pin_mut, select, FutureExt, StreamExt as _};
use log::{debug, info, trace, warn};
use smol::{process::Command as SmolCommand, Task, Timer};

use cargo_fixture::rpc_socket::{ConnectionType, Request, Response, RpcSocket};

use crate::{
    config::Config,
    utils::{CommandExt as _, CtrlC},
};

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

    pub async fn run(mut self) -> Result<i32> {
        let timer = smol::spawn(Self::watcher());
        let (socket, conn_type) = self
            .ctrlc
            .interruptible(self.socket.accept())
            .await
            .context("Fixture connection error")?;
        timer.cancel().await;
        if conn_type != ConnectionType::Fixture {
            bail!("Unexpected connection {conn_type:?}, expected fixture connection first");
        }

        let fixture = FixtureConnection::new(socket, self.config.clone(), self.kv_store.clone());
        let mut fixture = smol::spawn(fixture.run()).fuse();
        let mut ctrlc = self.ctrlc.clone();

        loop {
            let conn = self.socket.accept().fuse();
            pin_mut!(conn);
            select! {
                res = conn => self.handle_test_connection(res?).await?,
                res = fixture => return res,
                res = ctrlc => res?,
            }
        }
    }

    async fn watcher() {
        let start = Instant::now();
        let mut timer = Timer::interval(Duration::from_secs(10));
        while timer.next().await.is_some() {
            let delta = start.elapsed().as_secs();
            warn!("fixture process has been running for {delta}s but has not connected yet");
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
    replace_exec: Vec<String>,
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
            replace_exec: vec![],
            return_status: None,
        }
    }

    async fn run(mut self) -> Result<i32> {
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
                Request::Ready => self.handle_ready().await,
                hello @ Request::Hello { .. } => bail!("Unexpected Hello message: {hello:?}"),
            };
            self.socket.send(resp).await?;

            if let Some(status) = self.return_status {
                return Ok(status);
            }
        }
    }

    fn handle_set_env(&self, name: String, value: String) -> Response {
        debug!("setting env var {name}={value}");
        // FIXME: panics, see docs on when
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

    async fn handle_ready(&mut self) -> Response {
        let success = self.run_tests().await;
        Response::TestsFinished { success }
    }

    async fn run_tests(&mut self) -> bool {
        trace!("KV storage: {:?}", *self.kv_store.read().unwrap());

        let extra_test_args = mem::take(&mut self.extra_test_args);
        let extra_harness_args = mem::take(&mut self.extra_harness_args);
        let replace_exec = mem::take(&mut self.replace_exec);
        let test_cmd = self
            .config
            .test_cmd(extra_test_args, extra_harness_args, replace_exec);
        info!("running {}", test_cmd.display());
        let mut test_cmd = SmolCommand::from(test_cmd);
        test_cmd
            .status()
            .await
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
