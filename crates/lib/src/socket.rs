use std::path::Path;

use serde::{de::DeserializeOwned, Serialize};

// Unix
#[cfg(all(unix, feature = "smol"))]
use smol::net::unix::UnixStream;
#[cfg(all(unix, not(any(feature = "smol", feature = "tokio"))))]
use std::os::unix::net::UnixStream;
#[cfg(all(unix, feature = "tokio"))]
use tokio::net::UnixStream;

// Windows  TODO:
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html
                               // TODO: will need to be wrapped or something? Or converted to TcpStream?

// Platform common
#[cfg(feature = "smol")]
use smol::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
#[cfg(not(any(feature = "smol", feature = "tokio")))]
use std::io::{BufRead as _, BufReader, Write as _};
#[cfg(feature = "tokio")]
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};

use crate::{
    utils::{maybe_async, maybe_await},
    Error, Result,
};

#[derive(Debug)]
pub struct Socket {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl Socket {
    #[maybe_async]
    pub fn connect(socket_path: &Path) -> Result<Self> {
        let socket = maybe_await!(UnixStream::connect(socket_path)).map_err(Error::RpcIo)?;
        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Ok(Self { socket, buffer })
    }

    #[maybe_async]
    pub fn send<T>(&mut self, msg: T) -> Result<()>
    where
        T: Serialize,
    {
        let mut msg = serde_json::to_string(&msg).map_err(Error::RpcSerde)?;
        msg.push('\n');
        maybe_await!(self.socket.get_mut().write_all(msg.as_bytes())).map_err(Error::RpcIo)?;
        Ok(())
    }

    #[maybe_async]
    pub fn recv<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.buffer.clear();
        let num_read =
            maybe_await!(self.socket.read_line(&mut self.buffer)).map_err(Error::RpcIo)?;
        if num_read == 0 {
            Err(Error::RpcHangup)
        } else {
            serde_json::from_str(&self.buffer.trim()).map_err(Error::RpcSerde)
        }
    }
}
