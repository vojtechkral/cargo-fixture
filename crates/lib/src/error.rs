//! The library error type.

use std::{fmt, io};

use strum::AsRefStr;
use thiserror::Error;

/// Convenience `Result` alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The library error type.
#[derive(Error, AsRefStr)]
pub enum Error {
    /// RPC communication error.
    #[error("cargo fixture socket serde error")]
    RpcSerde(#[source] serde_json::Error),

    /// RPC I/O error.
    #[error("cargo fixture socket I/O error")]
    RpcIo(#[source] io::Error),

    /// No `CARGO_FIXTURE_SOCKET` set, occurs when fixture or fixture tests are run without `cargo fixture`.
    #[error("Could not connect: CARGO_FIXTURE_SOCKET not set; cargo fixture not running?")]
    RpcNoEnvVar,

    /// RPC communication error.
    #[error("Unexpected RPC response: {0:?}")]
    RpcMismatch(crate::rpc_socket::Response),

    /// Connection interrupted prematurely.
    #[error("cargo fixture socket unexpectedly hung up")]
    RpcHangup,

    /// Serde error when setting or retrieving K-V store value.
    #[error("De/serialization error")]
    Serde(#[from] serde_json::Error),

    /// Invalid key or value while attempting to set environment variable
    #[error("Invalid key or value while attempting to set environment variable")]
    InvalidSetEnv,

    /// No value set for key in the K-V store value.
    #[error("No value set for key `{0}`")]
    MissingKeyValue(String),
}

impl Error {
    /// Returns `Some` if this is a K-V store serde error.
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
