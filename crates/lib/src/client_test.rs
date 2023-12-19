use serde::de::DeserializeOwned;

use crate::{
    rpc_socket::{Request, RpcSocket},
    Result,
};

pub struct TestClient {
    socket: RpcSocket,
}

impl TestClient {
    pub async fn get_value<T>(&mut self, key: impl Into<String>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let req = Request::GetKeyValue { key: key.into() };
        let value = self.socket.call(req).await?.as_value()?;
        serde_json::from_value(value).map_err(Into::into)
    }
}
