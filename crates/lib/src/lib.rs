// TODO: deny missing docs

mod client_fixture;
mod client_test;
#[doc(hidden)]
pub mod data; // TODO: rm
pub mod error;
pub mod rpc; // TODO: rm
#[doc(hidden)]
pub mod rpc_socket;
#[doc(hidden)]
pub mod socket; // TODO: rm
mod utils;

pub use cargo_fixture_macros::with_fixture;
pub use client_fixture::FixtureClient;
pub use client_test::TestClient;
pub use error::{Error, Result};
pub use rpc::Client as Fixture; // TODO: rm
