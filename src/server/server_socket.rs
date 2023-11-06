use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Ok, Result};
use log::{trace, Level};

use cargo_fixture::rpc_socket::{platform::*, ConnectionType, Request, Response, RpcSocket};

use crate::utils::RmGuard;

#[derive(Debug)]
pub struct ServerSocket {
    socket: UnixListener,
    /// Ensure socket file is removed on server shutdown
    _rm_guard: RmGuard<PathBuf>,
}

impl ServerSocket {
    pub fn new(socket_path: &Path) -> Result<Self> {
        trace!("accepting connections on {}", socket_path.display());
        let rm_guard = RmGuard::new(socket_path.to_owned(), Level::Trace);
        let socket = UnixListener::bind(socket_path)
            .with_context(|| format!("Could not create a socket at {}", socket_path.display()))?;
        Ok(Self {
            socket,
            _rm_guard: rm_guard,
        })
    }

    pub async fn accept(&self) -> Result<(RpcSocket, ConnectionType)> {
        let (socket, _addr) = self
            .socket
            .accept()
            .await
            .context("Error accepting connection")?;
        trace!("connection accepted, performing handshake...");

        // Perform a connection handshake
        let mut socket = RpcSocket::new(socket);
        let our_ver = env!("CARGO_PKG_VERSION_MAJOR").parse::<u32>().unwrap();
        let msg = socket
            .recv()
            .await?
            .ok_or_else(|| anyhow!("Connection closed before handshake"))?;
        let connection_type = match msg {
            Request::Hello {
                version,
                connection_type,
            } if version == our_ver => connection_type,

            Request::Hello {
                version: theirs, ..
            } => {
                bail!("This cargo-fixture binary version ({our_ver}.x.y) is not compatible with the library linked by test code ({theirs}.x.y)")
            }

            other => bail!("Expected Hello message, got {other:?}"),
        };
        socket.send(Response::Ok).await?;

        trace!("connection handshake ok ({connection_type:?})");
        Ok((socket, connection_type))
    }
}
