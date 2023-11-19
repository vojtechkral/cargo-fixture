//! FIXME: doc-comment

use std::{
    fs::File,
    io::{self, BufRead as _, BufReader, Lines, StdinLock, StdoutLock, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum PipeRequest {
    SetEnv { name: String, value: String },
    EnqueueData { key: String, path: PathBuf },
    Ready,
    Finalize,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum PipeResponse {
    Ok,
    TestsFinished { success: bool },
}

impl PipeResponse {
    fn unwrap_ok(self) {
        assert!(matches!(self, Self::Ok), "Unexpected response: {self:?}");
    }

    // TODO: proper err type
    fn unwrap_tests_finished(self) -> Result<(), ()> {
        match self {
            PipeResponse::TestsFinished { success: true } => Ok(()),
            PipeResponse::TestsFinished { success: false } => Err(()),
            _ => panic!("Unexpected response: {self:?}"),
        }
    }
}

// FIXME: unwraps -> expects / lib error type?

pub struct Client {
    stdin: Lines<BufReader<StdinLock<'static>>>,
    stdout: StdoutLock<'static>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(io::stdin().lock()).lines(),
            stdout: io::stdout().lock(),
        }
    }

    fn call(&mut self, request: PipeRequest) -> PipeResponse {
        let request = serde_json::to_string(&request).unwrap();
        self.stdout.write_all(request.as_bytes()).unwrap();
        self.stdout.write_all(b"\n").unwrap();
        let response = self.stdin.next().unwrap().unwrap();
        serde_json::from_str(&response).unwrap()
    }

    pub fn set_env_var(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let req = PipeRequest::SetEnv {
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
        let req = PipeRequest::EnqueueData {
            key: key.into(),
            path,
        };
        self.call(req).unwrap_ok()
    }

    pub fn ready(&mut self) -> Result<(), ()> {
        self.call(PipeRequest::Ready).unwrap_tests_finished()
    }

    pub fn finalize(mut self) {
        self.call(PipeRequest::Finalize).unwrap_ok()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::PipeRequest;

    // TODO: the same for response
    #[test]
    fn pipe_request_serde() {
        let msg = PipeRequest::SetEnv {
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
