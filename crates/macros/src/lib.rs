extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;
use types::{Args, TestFn, WrappedFn};

mod types;

macro_rules! tri {
    ($what:expr) => {
        match $what {
            Ok(what) => what,
            Err(err) => return proc_macro::TokenStream::from(err.to_compile_error()),
        }
    };
}

/// Annotate a test function as a fixture test.
///
/// The attribute marks the function with an appropriate `cfg` attribute to be `#[ignore]`d when the `_fixture` feature is not active,
/// i.e. when not running under `cargo fixture`.
/// It also wraps the function with a `TestClient` connection.
///
/// The function's signature must be:
///
/// ```rust
/// async fn foo(client: TestClient)
/// ```
///
/// `#[with_fixture]` must come _before_ attributes like `#[tokio::test]`.
///
/// ### Serial connection
/// To have the `TestClient` connected with `serial` set to `true`, use the `serial` syntax:
///
/// ```rust
/// #[with_fixture(serial)]
/// ```
///
/// ## Example
///
/// ```
/// #[with_fixture]
/// #[tokio::test]
/// async fn with_fixture_example(mut client: TestClient) {
///     let example: Value = client.get_value("example").await.unwrap();
/// }
/// ```
#[proc_macro_attribute]
pub fn with_fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let test_fn = parse_macro_input!(input as TestFn);
    tri!(WrappedFn::wrap(test_fn, args))
        .into_token_stream()
        .into()
}
