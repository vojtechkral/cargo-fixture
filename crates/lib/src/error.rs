use std::{fmt, io, path::PathBuf};

use strum::AsRefStr;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, AsRefStr)]
pub enum Error {
    #[error("cargo fixture socket serde error")]
    RpcSerde(#[source] serde_json::Error),

    #[error("cargo fixture socket I/O error")]
    RpcIo(#[source] io::Error),

    #[error("Could not connect: CARGO_FIXTURE_SOCKET not set; cargo fixture not running?")]
    RpcNoEnvVar,

    #[error("Unexpected RPC response: {0:?}")]
    RpcMismatch(crate::rpc_socket::Response),

    #[error("cargo fixture socket unexpectedly hung up")]
    RpcHangup,

    #[error("De/serialization error")]
    Serde(#[from] serde_json::Error),

    #[error("No file found for key `{0}` (path: {1})")]
    DataFileNotFound(String, PathBuf),

    #[error("I/O error")]
    GeneralIo(#[from] io::Error),

    #[error("No value set for key `{0}`")]
    MissingKeyValue(String),
}

impl Error {
    pub fn as_serde(&self) -> Option<&serde_json::Error> {
        if let Self::Serde(err) = self {
            Some(err)
        } else {
            None
        }
    }
}

impl<T> From<Error> for Result<T> {
    fn from(err: Error) -> Self {
        Err(err)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant = self.as_ref();
        write!(f, "{variant}: {self}")
    }
}
