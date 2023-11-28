//! FIXME: doc-comment

use std::{env, fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{socket::Socket, Error, Result, utils::{maybe_await, maybe_async}};

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum Request {
    SetEnv { name: String, value: String },
    EnqueueData { key: String, path: PathBuf },
    Ready,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum Response {
    Ok,
    TestsFinished { success: bool },
}

impl Response {
    fn as_ok(self) -> Result<()> {
        match self {
            Self::Ok => Ok(()),
            _ => Error::RpcMismatch(self).into(),
        }
    }

    fn as_tests_finished(self) -> Result<bool> {
        match self {
            Response::TestsFinished { success } => Ok(success),
            _ => Error::RpcMismatch(self).into(),
        }
    }
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct WithVersion {
    pub ver: u32,

    #[serde(flatten)]
    pub request: Request,
}

impl WithVersion {
    pub fn new(ver: u32, request: Request) -> Self {
        Self { ver, request }
    }
}

pub struct Client {
    socket: Socket,
    version: u32,
}

impl Client {
    #[maybe_async]
    pub fn connect() -> Result<Self> {
        let socket_path =
            PathBuf::from(env::var_os("CARGO_FIXTURE_SOCKET").ok_or(Error::RpcNoEnvVar)?);
        Ok(Self {
            socket: maybe_await!(Socket::connect(&socket_path))?,
            version: env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap(),
        })
    }

    #[maybe_async]
    fn call(&mut self, request: Request) -> Result<Response> {
        maybe_await!(self.socket.send(WithVersion::new(self.version, request)))?;
        maybe_await!(self.socket.recv())
    }

    #[maybe_async]
    pub fn set_env_var(&mut self, name: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let req = Request::SetEnv {
            name: name.into(),
            value: value.into(),
        };

        maybe_await!(self.call(req))?.as_ok()
    }

    #[doc(hidden)]
    #[maybe_async]
    /// Not public API, please use the `get/set_fixture_data` macros.
    pub fn set_fixture_data(
        &mut self,
        key: impl Into<String>,
        path: PathBuf,
        value: impl Serialize,
    ) -> Result<()> {
        let file = File::create(&path)?;
        serde_json::to_writer_pretty(file, &value).map_err(Error::DataSerde)?;
        let req = Request::EnqueueData {
            key: key.into(),
            path,
        };
        maybe_await!(self.call(req))?.as_ok()
    }

    #[maybe_async]
    pub fn ready(&mut self) -> Result<bool> {
        maybe_await!(self.call(Request::Ready))?.as_tests_finished()
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
