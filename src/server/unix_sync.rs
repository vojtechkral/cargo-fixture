#![cfg(unix)]

use std::{os::unix::net::UnixListener, env, process};

pub struct Socket(UnixListener);

impl Socket {
    pub fn new() -> Self {
        let mut path = env::temp_dir();
        path.push(format!("cargo-fixture-{}", process::id()));
        let socket = UnixListener::bind(path).unwrap();
        Self(socket)
    }
}
