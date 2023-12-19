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
        let mut socket = RpcSocket::connect().await?;
        let version = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
        let connection_type = ConnectionType::Fixture;
        socket
            .call(Request::Hello {
                version,
                connection_type,
            })
            .await?
            .as_ok()?;
        // socket.call(Request::SetEnv { name: "Foo".into(), value: "value".into() }).await?.as_ok()?;
        Ok(Self { socket })
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

    pub async fn ready(&mut self) -> Result<bool> {
        self.socket.call(Request::Ready).await?.as_tests_finished()
    }
}
