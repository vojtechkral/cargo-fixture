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

/// FIXME: docs
#[proc_macro_attribute]
pub fn with_fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let test_fn = parse_macro_input!(input as TestFn);
    tri!(WrappedFn::wrap(test_fn, args))
        .into_token_stream()
        .into()
}
