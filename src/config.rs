use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio, self},
};

use crate::cli::Cli;

use self::cargo_meta::CargoMetadata;

mod cargo_meta;

#[derive(Debug)]
pub struct Config {
    pub cli: Cli,
    pub cargo_exe: PathBuf,
    pub socket_path: PathBuf,
}

impl Config {
    pub fn new(cli: Cli) -> Self {
        let cargo_exe: PathBuf = env::var_os("CARGO")
            .unwrap_or_else(|| {
                env::set_var("CARGO", "cargo");
                OsString::from("cargo")
            })
            .into();

        let metadata = CargoMetadata::read(&cargo_exe, &cli.args.cargo_flags_common);

        let target_dir = metadata.target_dir().clone();
        let pid = process::id();
        let socket_path = target_dir.join(&format!(".cargo-fixture-{pid}.sock"));

        Self {
            cli,
            cargo_exe,
            socket_path,
        }
    }

    pub fn fixture_cmd(&self) -> Command {
        let mut cmd = Command::new(self.cargo_exe.clone());
        cmd.arg("test")
            .args(&self.cli.args.cargo_flags_common)
            .args(["--test", "fixture", "--"])
            .args(&self.cli.fixture_args)
            // .env("CARGO_FIXTURE_SOCKET", val)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        cmd
    }

    pub fn test_cmd(&self) -> Command {
        let mut cmd = if let Some(exec) = self.cli.exec.get(0) {
            let mut cmd = Command::new(exec);
            cmd.args(&self.cli.exec[1..]);
            cmd
        } else {
            let mut cmd = Command::new(self.cargo_exe.clone());
            // NB. --features is additive
            // TODO: configurable feature name?
            cmd.args(["test", "--features", "fixture"]);
            cmd.args(&self.cli.args.args);
            cmd
        };
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        cmd
    }
}
