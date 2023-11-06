use serde::de::DeserializeOwned;

use crate::{
    rpc_socket::{ConnectionType, Request, RpcSocket},
    Result,
};

/// An RPC client used from test code.
///
/// An instance is created using [`TestClient::connect()`],
/// it's more convenient to use the [`with_fixture`][crate::with_fixture] macro.
pub struct TestClient {
    socket: RpcSocket,
}

impl TestClient {
    /// Connect to running `cargo fixture` process.
    ///
    /// The `serial` argument is a way to create a serial test. When set to `true`,
    /// `cargo fixture` will make sure that no other test client is connected at the same time.
    /// That is, if any other tests are already running, it will wait for them to finish,
    /// then let this connection proceed, and only let other connections in once this one is finished.
    pub async fn connect(serial: bool) -> Result<Self> {
        RpcSocket::connect(ConnectionType::client(serial))
            .await
            .map(|socket| Self { socket })
    }

    /// Get a copy of a value from `cargo fixture`'s in-memory K-V store.
    ///
    /// The value expected to have been prepared by the fixture. It can be any serde-serializable value.
    pub async fn get_value<T>(&mut self, key: impl Into<String>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let req = Request::GetKeyValue { key: key.into() };
        let value = self.socket.call(req).await?.as_value()?;
        serde_json::from_value(value).map_err(Into::into)
    }
}
