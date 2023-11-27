use std::{io, path::PathBuf};

use thiserror::Error;

use crate::rpc::Response;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cargo fixture IPC serde error")]
    RpcSerde(#[source] serde_json::Error),

    #[error("cargo fixture IPC I/O error")]
    RpcIo(#[source] io::Error),

    #[error("Could not connect: CARGO_FIXTURE_SOCKET not set. cargo fixture not running?")]
    RpcNoEnvVar,

    #[error("unexpected response from cargo fixture: {0:?}")]
    RpcMismatch(Response),

    #[error("fixture data serde error")]
    DataSerde(#[source] serde_json::Error),

    #[error("no file found for key `{0}` (path: {1})")]
    DataFileNotFound(String, PathBuf),

    #[error("I/O error")]
    GeneralIo(#[from] io::Error),
}

impl<T> From<Error> for Result<T> {
    fn from(err: Error) -> Self {
        Err(err)
    }
}
