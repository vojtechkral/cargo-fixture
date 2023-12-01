extern crate proc_macro;

use std::mem;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, Error, FnArg, ItemFn, Signature, Token};
use types::{TestFn, WrapFn};

mod types;

/// FIXME: docs
#[proc_macro_attribute]
pub fn with_fixture(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TestFn);
    let wrapped = WrapFn::new(input)
        .map(|w| w.into_token_stream())
        .unwrap_or_else(Error::into_compile_error);

    quote! {
        #[cfg_attr(not(feature = "fixture"), ignore = "only run under cargo fixture")]
        #wrapped
    }
    .into()
}

/// Implementation detail of `cargo-fixture-lib`.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn maybe_async(_args: TokenStream, input: TokenStream) -> TokenStream {
    let fun = parse_macro_input!(input as ItemFn);
    if fun.sig.asyncness.is_some() {
        return Error::new(
            fun.sig.fn_token.span,
            "fn with #[async_if] should be declared non-async",
        )
        .to_compile_error()
        .into();
    }

    let mut fun_async = fun.clone();
    fun_async.sig.asyncness = Some(Token![async](fun.sig.fn_token.span));

    quote! {
        #[cfg(any(feature = "smol", feature = "tokio"))]
        #fun_async
        #[cfg(not(any(feature = "smol", feature = "tokio")))]
        #fun
    }
    .into()
}
