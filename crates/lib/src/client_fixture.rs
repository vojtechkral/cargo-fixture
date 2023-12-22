use serde::Serialize;

use crate::{
    rpc_socket::{ConnectionType, Request, RpcSocket},
    Result,
};

pub struct FixtureClient {
    socket: RpcSocket,
}

impl FixtureClient {
    pub async fn connect() -> Result<Self> {
        RpcSocket::connect(ConnectionType::Fixture)
            .await
            .map(|socket| Self { socket })
    }

    pub async fn set_env_var(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let req = Request::SetEnv {
            name: name.into(),
            value: value.into(),
        };

        self.socket.call(req).await?.as_ok()
    }

    pub async fn set_extra_cargo_test_args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExtraTestArgs {
            args: args.into_iter().map(Into::into).collect(),
        };
        self.socket.call(req).await?.as_ok()
    }

    pub async fn set_extra_test_binary_args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExtraHarnessArgs {
            args: args.into_iter().map(Into::into).collect(),
        };
        self.socket.call(req).await?.as_ok()
    }

    pub async fn set_value(&mut self, key: impl Into<String>, value: impl Serialize) -> Result<()> {
        let value = serde_json::to_value(value)?;
        let req = Request::SetKeyValue {
            key: key.into(),
            value,
        };
        self.socket.call(req).await?.as_ok()
    }

    pub async fn set_exec(
        &mut self,
        exec: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExec {
            exec: exec.into_iter().map(Into::into).collect::<Vec<_>>(),
        };
        self.socket.call(req).await?.as_ok()
    }

    pub async fn ready(&mut self) -> Result<bool> {
        self.socket.call(Request::Ready).await?.as_tests_finished()
    }
}
