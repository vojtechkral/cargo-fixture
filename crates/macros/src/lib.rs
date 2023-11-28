extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Error, ItemFn, Token};

/// FIXME: docs
#[proc_macro_attribute]
pub fn with_fixture(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);

    quote! {
        #[cfg_attr(not(feature = "fixture"), ignore = "only run under cargo fixture")]
        #input
    }
    .into()
}

#[doc(hidden)]
#[proc_macro_attribute]
pub fn async_if(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
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
        #[cfg( #args )]
        #fun_async
        #[cfg(not( #args ))]
        #fun
    }
    .into()
}
