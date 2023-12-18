use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok, Result, anyhow};
use cargo_fixture::rpc_socket::{RpcSocket, ConnectionType, Request, Response};
use log::{trace, Level};
use serde::{de::DeserializeOwned, Serialize};
// FIXME: Windows
use smol::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    net::unix::{UnixListener, UnixStream},
};
// https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

use crate::utils::RmGuard;

#[derive(Debug)]
pub struct Socket {
    socket: UnixListener,
    /// Ensure socket file is removed as soon as not necessary
    _rm_guard: RmGuard<PathBuf>,
}

impl Socket {
    pub fn new(socket_path: &Path) -> Result<Self> {
        trace!("waiting for a connection on {}", socket_path.display());
        let rm_guard = RmGuard::new(socket_path.to_owned(), Level::Trace);
        let socket = UnixListener::bind(socket_path)
            .with_context(|| format!("Could not create a socket at {}", socket_path.display()))?;
        Ok(Self {
            socket,
            _rm_guard: rm_guard,
        })
    }

    pub async fn accept(self) -> Result<(RpcSocket, ConnectionType)> {
        let (socket, _addr) = self
            .socket
            .accept()
            .await
            .context("Error accepting fixture connection")?;

        let socket = RpcSocket::new(socket);
        socket.handle_request(|req| async {
            if let Request::Hello { version, connection_type } = req {
                // FIXME: check version
                Ok((Response::Ok, connection_type))
            } else {
                return Err(anyhow!(""));
            }
        })?;

        trace!("connection accepted");

        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Ok(Connection { socket, buffer })
    }
}
