//! FIXME: doc-comment

use std::{env, fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::socket::Socket;

// FIXME: rename these

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
    fn unwrap_ok(self) {
        assert!(matches!(self, Self::Ok), "Unexpected response: {self:?}");
    }

    // TODO: proper err type
    fn unwrap_tests_finished(self) -> Result<(), ()> {
        match self {
            Response::TestsFinished { success: true } => Ok(()),
            Response::TestsFinished { success: false } => Err(()),
            _ => panic!("Unexpected response: {self:?}"),
        }
    }
}

// FIXME: unwraps -> expects / lib error type?

pub struct Client {
    socket: Socket,
}

impl Client {
    pub fn connect() -> Self {
        let socket_path = PathBuf::from(env::var_os("CARGO_FIXTURE_SOCKET").expect("TODO:"));

        Self {
            socket: Socket::connect(&socket_path),
        }
    }

    fn call(&mut self, request: Request) -> Response {
        self.socket.send(request);
        self.socket.recv()
    }

    pub fn set_env_var(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let req = Request::SetEnv {
            name: name.into(),
            value: value.into(),
        };
        self.call(req).unwrap_ok();
    }

    #[doc(hidden)]
    /// Not public API, please use the `get/set_fixture_data` macros.
    pub fn set_fixture_data(
        &mut self,
        key: impl Into<String>,
        path: PathBuf,
        value: impl Serialize,
    ) {
        let file = File::create(&path).unwrap();
        serde_json::to_writer_pretty(file, &value).unwrap();
        let req = Request::EnqueueData {
            key: key.into(),
            path,
        };
        self.call(req).unwrap_ok()
    }

    pub fn ready(&mut self) -> Result<(), ()> {
        self.call(Request::Ready).unwrap_tests_finished()
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
