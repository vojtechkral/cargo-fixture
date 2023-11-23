use std::{path::{Path, PathBuf}, fmt::Debug};

use anyhow::{Result, Context, Ok};
use log::trace;
use serde::{de::DeserializeOwned, Serialize};
// FIXME: Windows
use smol::{net::unix::{UnixStream, UnixListener}, io::{BufReader, AsyncWriteExt as _, AsyncBufReadExt as _}};
// https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

use crate::utils::RmGuard;

#[derive(Debug)]
pub struct Socket {
    socket: UnixListener,
    /// Ensure socket file is removed as soon as not necessary
    rm_guard: RmGuard<PathBuf>,
}

pub struct Connection {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl Socket {
    pub fn new(socket_path: &Path) -> Result<Self> {
        trace!("waiting for a connection on {}", socket_path.display());
        let rm_guard = RmGuard(socket_path.to_owned());
        let socket = UnixListener::bind(socket_path).with_context(|| format!("Could not create a socket at {}", socket_path.display()))?;
        Ok(Self { socket, rm_guard })
    }

    pub async fn accept(self) -> Result<Connection> {
        let (socket, _addr) = self.socket.accept().await.context("Error accepting fixture process connection")?;
        trace!("connection accepted");

        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Ok(Connection { socket, buffer })

        // FIXME: version handshake?
    }
}

impl Connection {
    // FIXME: trace logs

    pub async fn send<T>(&mut self, msg: T) -> Result<()>
    where
        T: Serialize + Debug,
    {
        let mut msg = serde_json::to_string(&msg).with_context(|| format!("Error serializing message: {:?}", &msg))?;
        trace!("socket send: {msg}");
        msg.push('\n');
        self.socket
            .get_mut()
            .write_all(msg.as_bytes())
            .await
            .context("Error writing fixture process socket")
    }

    pub async fn recv<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.buffer.clear();
        let num_read = self.socket.read_line(&mut self.buffer).await.context("Error reading fixture process socket")?;
        if num_read == 0 {
            // FIXME: EOF/hangup, handle
        }
        let msg = self.buffer.trim();
        trace!("socket recv: `{}`", msg);
        let msg = serde_json::from_str(&msg).with_context(|| format!("Error deserializing message: `{msg}`"))?;
        Ok(msg)
    }
}
