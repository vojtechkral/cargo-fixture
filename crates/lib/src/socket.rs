use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
};

use log::trace;
use serde::{de::DeserializeOwned, Serialize};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

#[derive(Debug)]
pub struct Socket {
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl Socket {
    pub fn connect(socket_path: &Path) -> Self {
        let socket = UnixStream::connect(socket_path).expect("TODO:");
        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Self { socket, buffer }
    }

    // FIXME: trace logs

    pub fn send<T>(&mut self, msg: T)
    where
        T: Serialize,
    {
        let mut msg = serde_json::to_string(&msg).expect("TODO:");
        trace!("socket send: {msg}");
        msg.push('\n');
        self.socket
            .get_mut()
            .write_all(msg.as_bytes())
            .expect("TODO:");
    }

    pub fn recv<T>(&mut self) -> T
    where
        T: DeserializeOwned,
    {
        self.buffer.clear();
        let num_read = self.socket.read_line(&mut self.buffer).expect("TODO:");
        if num_read == 0 {
            // EOF/hangup, handle
        }
        trace!("socket recv: `{}`", self.buffer.trim());
        serde_json::from_str(&self.buffer.trim()).expect("TODO:")
    }
}
