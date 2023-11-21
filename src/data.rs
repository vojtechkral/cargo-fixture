use std::{ffi::OsStr, fs::File, path::PathBuf};

use serde::de::DeserializeOwned;

// FIXME: unwraps

#[macro_export]
macro_rules! set_fixture_data {
    ($client:ident, $key:expr, $value:expr) => {
        $client.set_fixture_data(
            $key,
            $crate::data::format_path(
                $key,
                env!(
                    "CARGO_TARGET_TMPDIR",
                    "This macro can only be used in an integration test code."
                ),
            ),
            $value,
        )
    };
}

#[macro_export]
macro_rules! get_fixture_data {
    ($key:tt as  $ty:ty) => {
        $crate::data::get::<$ty>(
            $key,
            $crate::data::format_path(
                $key,
                env!(
                    "CARGO_TARGET_TMPDIR",
                    "This macro can only be used in an integration test code."
                ),
            ),
        )
    };
}

#[doc(hidden)]
/// Not public API, please use the `get/set_fixture_data` macros.
pub fn format_path(key: impl AsRef<str>, cargo_target_tmpdir: impl AsRef<OsStr>) -> PathBuf {
    let mut path = PathBuf::from(cargo_target_tmpdir.as_ref());
    path.push(format!("cargo-fixture-data-{}.json", key.as_ref()));
    path
}

#[doc(hidden)]
/// Not public API, please use the `get/set_fixture_data` macros.
pub fn get<T>(key: impl AsRef<str>, file: PathBuf) -> T
where
    T: DeserializeOwned,
{
    if !file.exists() {
        // FIXME: nice error
    }
    let file = File::open(&file).unwrap();
    serde_json::from_reader(file).unwrap()
}
