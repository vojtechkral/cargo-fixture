// TODO: deny missing docs

mod client_fixture;
mod client_test;
pub mod error;
#[doc(hidden)]
pub mod rpc_socket;

pub use cargo_fixture_macros::with_fixture;
pub use client_fixture::FixtureClient;
pub use client_test::TestClient;
pub use error::{Error, Result};
