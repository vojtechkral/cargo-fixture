use std::{
    ffi::OsStr,
    io::{self, Write as _},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use log::trace;
use serde::Deserialize;

use crate::utils::ExitStatusExt;

/// Subset of `cargo metadata` output. I'm not using the `cargo_metadata` crate
/// as it seems like an overkill for this and doesn't actually make my job easier for creating
/// the command.
#[derive(Deserialize, Debug)]
pub struct CargoMetadata {
    target_directory: PathBuf,
}

impl CargoMetadata {
    pub fn read(cargo: impl AsRef<OsStr>, flags: &[impl AsRef<OsStr>]) -> Result<Self> {
        trace!(
            "Running {} metadata --format-version 1 --no-deps",
            Path::new(cargo.as_ref()).display()
        );
        let output = Command::new(cargo)
            .arg("metadata")
            .args(flags)
            .args(["--format-version", "1", "--no-deps"])
            .output()
            .context("Could not run `cargo metadata`")?;

        let status = output.status.as_result("cargo metadata command failed");
        if status.is_err() {
            let _ = io::stderr().write_all(&output.stderr);
            status?
        }

        trace!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stdout[..])
        );
        let this = serde_json::from_slice(&output.stdout)
            .context("Failed to deserialize `cargo metadata` output")?;
        Ok(this)
    }

    pub fn target_dir(&self) -> &PathBuf {
        &self.target_directory
    }
}
