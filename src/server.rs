use anyhow::{Context, Result};
use async_ctrlc::CtrlC;

#[cfg(unix)]
use tokio::net::UnixStream;
// Windows  TODO:
#[cfg(windows)]
use uds_windows::UnixListener;

use crate::{config::Config, fixture::socket::Socket};

mod socket;

pub struct Server {
    config: Config,
    socket: Socket,
    ctrlc: CtrlC,
}

impl Server {
    pub fn new(config: Config) -> Result<Self> {
        let ctrlc = CtrlC::new().context("Failed to create SIGINT handler")?;
        let socket = Socket::new(&config.socket_path)?;
        Ok(Self {
            config,
            socket,
            ctrlc,
        })
    }

    pub async fn run(self) -> Result<()> {
        let conn = self.socket.accept()

        todo!()
    }

    async fn accept(&self) -> Result<UnixStream>
}
