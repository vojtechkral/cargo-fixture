use std::process::Command;

use anyhow::Result;

pub fn run_tests(cargo: &str, args: &[String]) -> Result<bool> {
    let mut cmd = Command::new(cargo);
    let cmd = if args.is_empty() {
        cmd.arg("test")
    } else {
        cmd.args(args)
    };
    cmd.status()
        .map(|status| status.success())
        .map_err(Into::into)
}
