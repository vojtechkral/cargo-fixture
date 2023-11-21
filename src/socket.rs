use std::{path::Path, time::Duration, io::{BufReader, BufRead}};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use log::warn;
use serde::de::DeserializeOwned;
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

use cargo_fixture::rpc::{PipeRequest, PipeResponse};

pub struct Socket{
    socket: BufReader<UnixStream>,
    buffer: String,
}

impl Socket {
    pub fn accept(socket_path: &Path) -> Self {
        let listener = UnixListener::bind(socket_path).expect("TODO:");
        let (socket, _addr) = listener.accept().expect("TODO:");
        dbg!(_addr);
        socket.set_read_timeout(Some(Duration::from_millis(100))).expect("TODO:");
        let socket = BufReader::new(socket);
        let buffer = String::with_capacity(1024);
        Self{ socket, buffer }
    }

    pub fn recv<T>(&mut self) -> T where T: DeserializeOwned {
        warn!("reading...");
        let line = self.socket.read_line(&mut self.buffer);//.expect("TODO:");
        dbg!(line);
        todo!()
    }
}
