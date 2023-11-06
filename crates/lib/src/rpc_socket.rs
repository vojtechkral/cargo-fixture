//! FIXME: doc-comment

use std::{env, path::PathBuf};

use log::trace;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{Error, Result};

pub mod platform;
use platform::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionType {
    Fixture,
    Client,
    ClientSerial,
}

impl ConnectionType {
    pub fn client(serial: bool) -> Self {
        if serial {
            Self::ClientSerial
        } else {
            Self::Client
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum Request {
    Hello {
        version: u32,
        connection_type: ConnectionType,
    },
    SetEnv {
        name: String,
        value: String,
    },
    // TODO: naming of this stuff?
    SetKeyValue {
        key: String,
        value: serde_json::Value,
    },
    GetKeyValue {
        key: String,
    },
    SetExtraTestArgs {
        args: Vec<String>,
    },
    SetExtraHarnessArgs {
        args: Vec<String>,
    },
    SetExec {
        exec: Vec<String>,
    },
    Ready,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum Response {
    Ok,
    TestsFinished {
        success: bool,
    },
    KeyValue {
        key: String,
        value: Option<serde_json::Value>,
    },
}

impl Response {
    pub fn as_ok(self) -> Result<()> {
        match self {
            Self::Ok => Ok(()),
            _ => Error::RpcMismatch(self).into(),
        }
    }

    pub fn as_tests_finished(self) -> Result<bool> {
        match self {
            Response::TestsFinished { success } => Ok(success),
            _ => Error::RpcMismatch(self).into(),
        }
    }

    pub fn as_value(self) -> Result<serde_json::Value> {
        match self {
            Response::KeyValue { key, value } => value.ok_or(Error::MissingKeyValue(key)),
            _ => Error::RpcMismatch(self).into(),
        }
    }
}

// TODO: use interior mutability? so that all uses don't have to be mut
#[derive(Debug)]
pub struct RpcSocket {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl RpcSocket {
    pub(crate) async fn connect(connection_type: ConnectionType) -> Result<Self> {
        let path = PathBuf::from(env::var_os("CARGO_FIXTURE_SOCKET").ok_or(Error::RpcNoEnvVar)?);
        let stream = UnixStream::connect(path).await.map_err(Error::RpcIo)?;
        let mut this = Self::new(stream);

        // Perform handshake
        let version = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
        this.call(Request::Hello {
            version,
            connection_type,
        })
        .await?
        .as_ok()?;
        Ok(this)
    }

    pub fn new(stream: UnixStream) -> Self {
        let socket = BufReader::new(stream);
        let buffer = String::with_capacity(1024);
        Self { socket, buffer }
    }

    pub async fn send<T>(&mut self, msg: T) -> Result<()>
    where
        T: Serialize,
    {
        let mut msg = serde_json::to_string(&msg).map_err(Error::RpcSerde)?;
        trace!("RPC send: {msg}");
        msg.push('\n');
        self.socket
            .get_mut()
            .write_all(msg.as_bytes())
            .await
            .map_err(Error::RpcIo)?;
        Ok(())
    }

    pub async fn recv<T>(&mut self) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.buffer.clear();
        let num_read = self
            .socket
            .read_line(&mut self.buffer)
            .await
            .map_err(Error::RpcIo)?;
        if num_read == 0 {
            Ok(None)
        } else {
            let msg = &self.buffer.trim();
            trace!("RPC recv: {msg}");
            serde_json::from_str(msg).map(Some).map_err(Error::RpcSerde)
        }
    }

    pub(crate) async fn call(&mut self, request: Request) -> Result<Response> {
        self.send(request).await?;
        self.recv().await?.ok_or(Error::RpcHangup)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Request;

    // TODO: the same for response
    #[test]
    fn pipe_request_serde() {
        let msg = Request::SetEnv {
            name: "FOO".to_string(),
            value: "bar/baz".to_string(),
        };
        let msg = serde_json::to_value(&msg).unwrap();

        let expected = json!({
            "msg": "SetEnv",
            "data": {"name": "FOO", "value": "bar/baz"}
        });
        assert_eq!(msg, expected);
    }
}
