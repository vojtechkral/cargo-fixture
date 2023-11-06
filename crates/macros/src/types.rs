use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Error, Result, Signature, Visibility,
};

mod kw {
    syn::custom_keyword!(serial);
}

pub struct Args {
    serial: Option<kw::serial>,
}

impl Args {
    fn serial_as_bool(&self) -> Ident {
        let value = if self.serial.is_some() {
            "true"
        } else {
            "false"
        };
        Ident::new(value, self.serial.span())
    }
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            serial: input.parse()?,
        })
    }
}

/// Like `ItemFn` but with the body not parsed (we don't need it).
///
/// NB. inner attributes on test fns are not supported by this
/// (sorry, they're a major PITA and no one actually uses them I think?).
pub struct TestFn {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
    pub block: TokenStream,
}

impl Parse for TestFn {
    #[inline]
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            sig: input.parse()?,
            block: input.parse()?,
        })
    }
}

impl ToTokens for TestFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attrs,
            vis,
            sig,
            block,
        } = self;

        attrs.iter().for_each(|attr| attr.to_tokens(tokens));
        vis.to_tokens(tokens);
        sig.to_tokens(tokens);
        tokens.extend([block.clone()]);
    }
}

pub struct WrappedFn {
    test_fn: TestFn,
    args: Args,
}

impl WrappedFn {
    pub fn wrap(test_fn: TestFn, args: Args) -> Result<Self> {
        if test_fn.sig.inputs.len() == 1 {
            Ok(Self { test_fn, args })
        } else {
            Err(Error::new_spanned(
                test_fn.sig.inputs,
                "a with_fixture test function has to take one argument of type TestClient",
            ))
        }
    }
}

impl ToTokens for WrappedFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.to_token_stream());
    }

    fn into_token_stream(self) -> TokenStream
    where
        Self: Sized,
    {
        let Self { mut test_fn, args } = self;
        let serial = args.serial_as_bool();

        // Generate the wrapping fn
        let mut wrapper_sig = test_fn.sig.clone();
        wrapper_sig.generics = Default::default();
        wrapper_sig.inputs = Default::default();
        let wrapper_attrs = test_fn.attrs.clone();
        test_fn.attrs = Default::default();
        let test_fn_ident = &test_fn.sig.ident;
        let wrapper_fn = TestFn {
            attrs: wrapper_attrs,
            vis: test_fn.vis.clone(),
            sig: wrapper_sig,
            block: quote! {{
                #test_fn
                let client = ::cargo_fixture::TestClient::connect(#serial)
                    .await
                    .expect("Could not connect to cargo fixture");
                #test_fn_ident(client).await
            }},
        };

        quote! {
            #[cfg_attr(not(feature = "_fixture"), ignore = "only ran under cargo fixture")]
            #wrapper_fn
        }
    }
}
