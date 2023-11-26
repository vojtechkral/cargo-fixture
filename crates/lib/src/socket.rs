use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
};

use serde::{de::DeserializeOwned, Serialize};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

use crate::{Error, Result};

#[derive(Debug)]
pub struct Socket {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl Socket {
    pub fn connect(socket_path: &Path) -> Result<Self> {
        let socket = UnixStream::connect(socket_path).map_err(Error::RpcIo)?;
        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Ok(Self { socket, buffer })
    }

    pub fn send<T>(&mut self, msg: T) -> Result<()>
    where
        T: Serialize,
    {
        let mut msg = serde_json::to_string(&msg).map_err(Error::RpcSerde)?;
        msg.push('\n');
        self.socket
            .get_mut()
            .write_all(msg.as_bytes())
            .map_err(Error::RpcIo)?;
        Ok(())
    }

    pub fn recv<T>(&mut self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.buffer.clear();
        let num_read = self
            .socket
            .read_line(&mut self.buffer)
            .map_err(Error::RpcIo)?;
        if num_read == 0 {
            // FIXME: EOF/hangup, handle
        }
        serde_json::from_str(&self.buffer.trim()).map_err(Error::RpcSerde)
    }
}
