use serde::de::DeserializeOwned;

use crate::{
    rpc_socket::{ConnectionType, Request, RpcSocket},
    Result,
};

pub struct TestClient {
    socket: RpcSocket,
}

impl TestClient {
    pub async fn connect(serial: bool) -> Result<Self> {
        RpcSocket::connect(ConnectionType::client(serial))
            .await
            .map(|socket| Self { socket })
    }

    pub async fn get_value<T>(&mut self, key: impl Into<String>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let req = Request::GetKeyValue { key: key.into() };
        let value = self.socket.call(req).await?.as_value()?;
        serde_json::from_value(value).map_err(Into::into)
    }
}
