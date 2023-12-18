#[doc(hidden)]
pub mod data;
pub mod error;
pub mod rpc;  // TODO: rm
#[doc(hidden)]
pub mod rpc_socket;
#[doc(hidden)]
pub mod socket;  // TODO: rm
mod utils;

pub use cargo_fixture_macros::with_fixture;

pub use error::{Error, Result};
pub use rpc::Client as Fixture;
