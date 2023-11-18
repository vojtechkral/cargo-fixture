//! FIXME: doc-comment

use std::io::{self, BufRead as _, BufReader, Lines, StdinLock, StdoutLock, Write};

use serde::{Deserialize, Serialize};

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "msg", content = "data")]
pub enum PipeRequest {
    SetEnv { name: String, value: String },
    RunTests { args: Option<Vec<String>> },  // FIXME: remove args, rename as Ready
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

    // TODO: return a result in these
    pub fn run_tests(&mut self) -> Result<(), ()> {
        self.call(PipeRequest::RunTests { args: None })
            .unwrap_tests_finished()
    }

    pub fn run_tests_args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<(), ()> {
        let args = args.into_iter().map(Into::into).collect();
        self.call(PipeRequest::RunTests { args: Some(args) })
            .unwrap_tests_finished()
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
