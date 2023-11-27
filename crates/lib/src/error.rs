use std::{io, path::PathBuf};

use thiserror::Error;

use crate::rpc::Response;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cargo fixture socket serde error")]
    RpcSerde(#[source] serde_json::Error),

    #[error("cargo fixture socket I/O error")]
    RpcIo(#[source] io::Error),

    #[error("Could not connect: CARGO_FIXTURE_SOCKET not set. cargo fixture not running?")]
    RpcNoEnvVar,

    #[error("Unexpected response from cargo fixture: {0:?}")]
    RpcMismatch(Response),

    #[error("cargo fixture socket unexpectedly hung up")]
    RpcHangup,

    #[error("Fixture data serde error")]
    DataSerde(#[source] serde_json::Error),

    #[error("No file found for key `{0}` (path: {1})")]
    DataFileNotFound(String, PathBuf),

    #[error("I/O error")]
    GeneralIo(#[from] io::Error),
}

impl Error {
    pub fn as_data_serde(&self) -> Option<&serde_json::Error> {
        if let Self::DataSerde(err) = self {
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
