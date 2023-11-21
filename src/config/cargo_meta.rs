use std::{ffi::OsStr, path::PathBuf, process::Command};

use serde::Deserialize;

/// Subset of `cargo metada` output. I'm not using the `cargo_metadata` crate
/// as it seems like an overkill for this and doesn't actually make my job easier for creating
/// the command.
#[derive(Deserialize, Debug)]
pub struct CargoMetadata {
    packages: Vec<Package>,
    target_directory: PathBuf,
}

impl CargoMetadata {
    pub fn read(cargo: impl AsRef<OsStr>, flags: &[impl AsRef<OsStr>]) -> Self {
        let output = Command::new(cargo)
            .args(flags)
            .args(["--format-version", "1", "--no-deps"])
            .output()
            .expect("TODO:");

        serde_json::from_slice(&output.stdout).expect("TODO:")
    }

    pub fn target_dir(&self) -> &PathBuf {
        &self.target_directory
    }
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    manifest_path: PathBuf,
}
