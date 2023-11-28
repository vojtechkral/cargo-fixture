#[doc(hidden)]
pub mod data;
pub mod error;
pub mod rpc;
#[doc(hidden)]
pub mod socket;
mod utils;

pub use cargo_fixture_macros::with_fixture;

pub use error::{Error, Result};
pub use rpc::Client as Fixture;
