extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

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
