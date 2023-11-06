#![deny(missing_docs)]

//! This is an accompanying library for the [`cargo fixture`](https://github.com/vojtechkral/cargo-fixture) cargo extension.
//!
//! The library provides two main types:
//! - [`FixtureClient`] &ndash; to be used from fixture code
//! - [`TestClient`] &ndash; to be used from test code
//!
//! The [`with_fixture`] macros is provided as well for easy fixture tests definition.
//!
//! ## Features
//! The library supports the following async runtimes, selectable with a feature of the same name:
//! - [`tokio`](https://tokio.rs/)
//! - [`smol`](https://docs.rs/smol)
//!
//! You have to activate exactly one of these features to use the library.

mod client_fixture;
mod client_test;
pub mod error;
#[doc(hidden)]
pub mod rpc_socket;

pub use cargo_fixture_macros::with_fixture;
pub use client_fixture::FixtureClient;
pub use client_test::TestClient;
pub use error::{Error, Result};
