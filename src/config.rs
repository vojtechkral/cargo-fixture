use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    process::{self, Command, Stdio},
};

mod cargo_meta;

use anyhow::Result;

use self::cargo_meta::CargoMetadata;
use crate::cli::Cli;

#[derive(Debug)]
pub struct Config {
    pub cli: Cli,
    pub cargo_exe: PathBuf,
    pub socket_path: PathBuf,
}

impl Config {
    pub fn new(cli: Cli) -> Result<Self> {
        let cargo_exe: PathBuf = env::var_os("CARGO")
            .unwrap_or_else(|| {
                env::set_var("CARGO", "cargo");
                OsString::from("cargo")
            })
            .into();

        let metadata = CargoMetadata::read(&cargo_exe, &cli.cargo_common_all)?;

        let target_dir = metadata.target_dir().clone();
        let pid = process::id();
        let socket_path = target_dir.join(&format!(".cargo-fixture-{pid}.sock"));

        Ok(Self {
            cli,
            cargo_exe,
            socket_path,
        })
    }

    pub fn fixture_cmd(&self) -> Command {
        let mut cmd = Command::new(self.cargo_exe.clone());
        cmd.arg("test")
            .args(&self.cli.cargo_common_test)
            .args(["--test", "fixture", "--"])
            .args(&self.cli.fixture_args)
            .env("CARGO_FIXTURE_SOCKET", &self.socket_path)
            .stdin(Stdio::null());
        cmd
    }

    pub fn test_cmd(
        &self,
        add_args_cargo_test: Vec<String>,
        add_args_harness: Vec<String>,
    ) -> Command {
        let mut cmd = if let Some(exec) = self.cli.exec.get(0) {
            let mut cmd = Command::new(exec);
            cmd.args(&self.cli.exec[1..]);
            cmd
        } else {
            let mut cmd = Command::new(self.cargo_exe.clone());
            // NB. --features is additive
            // TODO: configurable feature name? (nah)
            cmd.args(["test", "--features", "fixture"]);

            cmd.args(&self.cli.cargo_common_all)
                .args(add_args_cargo_test)
                .arg("--")
                .args(&self.cli.harness_args)
                .args(add_args_harness);
            cmd
        };
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        cmd
    }
}
