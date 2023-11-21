#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(windows)]
use uds_windows::UnixListener; // https://docs.rs/uds_windows/latest/uds_windows/struct.UnixListener.html

pub struct Server(UnixListener);

impl Server {
    pub fn new() -> Self {
        todo!()
    }

    // pub fn recv(&self) ->
}
