//! FIXME: doc-comment

use std::{env, path::PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

// Unix
#[cfg(all(unix, feature = "smol"))]
use smol::net::unix::UnixStream;
#[cfg(all(unix, not(feature = "smol"), feature = "tokio"))]
use tokio::net::UnixStream;

// Windows  TODO:
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html
                               // TODO: will need to be wrapped for async? Or converted to TcpStream?

// Platform common
#[cfg(feature = "smol")]
use smol::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
#[cfg(all(not(feature = "smol"), feature = "tokio"))]
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use crate::{Error, Result};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionType {
    Fixture,
    Client,
    ClientSerial,
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
            _ => Error::RpcMismatch_(self).into(),
        }
    }

    pub fn as_tests_finished(self) -> Result<bool> {
        match self {
            Response::TestsFinished { success } => Ok(success),
            _ => Error::RpcMismatch_(self).into(),
        }
    }

    pub fn as_value(self) -> Result<serde_json::Value> {
        match self {
            Response::KeyValue { key, value } => value.ok_or(Error::MissingKeyValue(key)),
            _ => Error::RpcMismatch_(self).into(),
        }
    }
}

#[derive(Debug)]
pub struct RpcSocket {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl RpcSocket {
    pub(crate) async fn connect() -> Result<Self> {
        let path = PathBuf::from(env::var_os("CARGO_FIXTURE_SOCKET").ok_or(Error::RpcNoEnvVar)?);
        let stream = UnixStream::connect(path).await.map_err(Error::RpcIo)?;
        Ok(Self::new(stream))
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
        msg.push('\n');
        self.socket
            .get_mut()
            .write_all(msg.as_bytes())
            .await
            .map_err(Error::RpcIo)?;
        Ok(())
    }

    pub async fn recv<T>(&mut self) -> Result<T>
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
            Err(Error::RpcHangup)
        } else {
            serde_json::from_str(&self.buffer.trim()).map_err(Error::RpcSerde)
        }
    }

    pub(crate) async fn call(&mut self, request: Request) -> Result<Response> {
        self.send(request).await?;
        self.recv().await
    }

    // FIXME: move these to FixtureClient

    // pub async fn set_env_var(
    //     &mut self,
    //     name: impl Into<String>,
    //     value: impl Into<String>,
    // ) -> Result<()> {
    //     let req = Request::SetEnv {
    //         name: name.into(),
    //         value: value.into(),
    //     };

    //     self.call(req).await?.as_ok()
    // }

    // pub async fn set_additional_cargo_test_args(
    //     &mut self,
    //     args: impl IntoIterator<Item = impl Into<String>>,
    // ) -> Result<()> {
    //     let to_cargo_test = Some(args.into_iter().map(Into::into).collect());
    //     let req = Request::SetAdditionalArgs {
    //         to_cargo_test,
    //         to_harness: None,
    //     };

    //     self.call(req).await?.as_ok()
    // }

    // pub async fn set_additional_harness_args(
    //     &mut self,
    //     args: impl IntoIterator<Item = impl Into<String>>,
    // ) -> Result<()> {
    //     let to_harness = Some(args.into_iter().map(Into::into).collect());
    //     let req = Request::SetAdditionalArgs {
    //         to_cargo_test: None,
    //         to_harness,
    //     };

    //     self.call(req).await?.as_ok()
    // }

    // TODO: rework (+ rename)
    // #[maybe_async]
    // /// Not public API, please use the `get/set_fixture_data` macros.
    // pub fn set_fixture_data(
    //     &mut self,
    //     key: impl Into<String>,
    //     path: PathBuf,
    //     value: impl Serialize,
    // ) -> Result<()> {
    //     let file = File::create(&path)?;
    //     serde_json::to_writer_pretty(file, &value).map_err(Error::DataSerde)?;
    //     let req = Request::EnqueueData {
    //         key: key.into(),
    //         path,
    //     };
    //     maybe_await!(self.call(req))?.as_ok()
    // }

    // pub async fn ready(&mut self) -> Result<bool> {
    //     self.call(Request::Ready).await?.as_tests_finished()
    // }
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
