use std::{
    net::IpAddr,
    process::{Command, Output},
};

use serde::{Deserialize, Serialize};

pub fn cargo_fixture() -> CargoFixture {
    let exe = env!("CARGO_BIN_EXE_cargo-fixture");
    CargoFixture {
        cmd: Command::new(exe),
    }
}

pub struct CargoFixture {
    cmd: Command,
}

impl CargoFixture {
    pub fn test(mut self, test: &str) -> Output {
        let fixture = format!("fixture_{test}");
        let callback = format!("{test}_callback");
        self.cmd
            .args([
                "--fixture",
                &fixture,
                "--",
                "--nocapture",
                "--exact",
                &callback,
            ])
            .output()
            .unwrap()
    }
}

pub trait OutputExt {
    fn assert_success(&self);
    fn assert_error(&self, stderr_contains: &str);
}

impl OutputExt for Output {
    fn assert_success(&self) {
        let success = self.status.success();
        if !success {
            let stderr = String::from_utf8_lossy(&self.stderr).replace('\n', "\n  ");
            eprintln!("cargo fixture stderr:\n\n  {stderr}");
        }
        assert!(success);
    }

    fn assert_error(&self, stderr_contains: &str) {
        assert!(!self.status.success());
        let stderr = String::from_utf8_lossy(&self.stderr);
        assert!(stderr.contains(stderr_contains));
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KvExample {
    pub foo: String,
    pub bar: IpAddr,
}
