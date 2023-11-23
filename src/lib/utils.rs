use std::{fs, path::Path, time::Duration, io, os::{unix::net::{UnixListener, UnixStream}, fd::OwnedFd}};

use log::trace;

// FIXME: rm

// pub trait UnixListenerExt: Sized {
//     fn set_accept_timeout(self, timeout: Option<Duration>) -> io::Result<Self>;
// }

// impl UnixListenerExt for UnixListener {
//     fn set_accept_timeout(self, timeout: Option<Duration>) -> io::Result<Self> {
//         // This is rather silly
//         let listener = UnixStream::from(OwnedFd::from(self));
//         listener.set_read_timeout(timeout)?;
//         Ok(OwnedFd::from(listener).into())
//     }
// }
