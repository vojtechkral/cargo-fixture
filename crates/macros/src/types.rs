use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token, Attribute, Error, Expr, ExprCall, FnArg, Pat, Result, Signature, Token, Visibility,
};

/// FIXME: mention tokio
pub struct TestFn {
    outer_attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    brace_token: token::Brace,
    inner_attrs: Vec<Attribute>,
    stmts: Vec<TokenStream>,
}

impl Parse for TestFn {
    #[inline]
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // This parse implementation has been largely lifted from `syn`, with
        // the exception of:
        // * We don't have access to the plumbing necessary to parse inner
        //   attributes in-place.
        // * We do our own statements parsing to avoid recursively parsing
        //   entire statements and only look for the parts we're interested in.

        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;

        let content;
        let brace_token = braced!(content in input);
        let inner_attrs = Attribute::parse_inner(&content)?;

        let mut buf = TokenStream::new();
        let mut stmts = Vec::new();

        while !content.is_empty() {
            if let Some(semi) = content.parse::<Option<syn::Token![;]>>()? {
                semi.to_tokens(&mut buf);
                stmts.push(buf);
                buf = proc_macro2::TokenStream::new();
                continue;
            }

            // Parse a single token tree and extend our current buffer with it.
            // This avoids parsing the entire content of the sub-tree.
            buf.extend([content.parse::<TokenTree>()?]);
        }

        if !buf.is_empty() {
            stmts.push(buf);
        }

        Ok(Self {
            outer_attrs,
            vis,
            sig,
            brace_token,
            inner_attrs,
            stmts,
        })
    }
}

impl ToTokens for TestFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.outer_attrs
            .iter()
            .for_each(|attr| attr.to_tokens(tokens));
        self.vis.to_tokens(tokens);
        self.sig.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            self.stmts.iter().for_each(|s| s.to_tokens(tokens));
        });
    }
}

pub struct WrapFn {
    test_fn: TestFn,
    outer_sig: Signature,
    call_expr: Expr,
}

impl WrapFn {
    pub fn new(mut test_fn: TestFn) -> Result<Self> {
        let fn_ident = &test_fn.sig.ident;
        let mut outer_sig = test_fn.sig.clone();
        outer_sig.inputs.clear();
        let mut call_args: Punctuated<Expr, Token![,]> = Punctuated::new();
        for arg in test_fn.sig.inputs.iter_mut() {
            // FIXME: attrs not removed
            let kind = arg.take_arg_kind()?;
            if kind.copy_to_outer() {
                outer_sig.inputs.push(arg.clone());
            }

            call_args.push(kind.into_call_arg());
        }

        let call_expr: ExprCall = parse_quote! {
            #fn_ident ( #call_args )
        };

        Ok(Self {
            test_fn,
            outer_sig,
            call_expr: Expr::Call(call_expr),
        })
    }
}

impl ToTokens for WrapFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            test_fn,
            outer_sig,
            call_expr,
        } = self;
        tokens.extend(quote! {
            #outer_sig {
                #test_fn
                #call_expr
            }
        });
    }
}

enum ArgKind {
    Env(Ident),
    TmpData(Ident),
    Passthrough(Ident),
    Receiver(Token![self]),
}

impl ArgKind {
    fn copy_to_outer(&self) -> bool {
        match self {
            Self::Env(_) | Self::TmpData(_) => false,
            Self::Passthrough(_) | Self::Receiver(_) => true,
        }
    }

    fn into_call_arg(self) -> Expr {
        match self {
            Self::Env(ident) => {
                let var = ident.to_string().to_uppercase();
                parse_quote! { ::std::env::var(#var).expect("FIXME: errmsg") }
            }
            Self::TmpData(ident) => todo!(),
            Self::Passthrough(ident) => parse_quote! { #ident },
            Self::Receiver(_) => parse_quote! { self },
        }
    }
}

trait FnArgExt {
    fn take_arg_kind(&mut self) -> Result<ArgKind>;
}

impl FnArgExt for FnArg {
    fn take_arg_kind(&mut self) -> Result<ArgKind> {
        match self {
            FnArg::Typed(typed) => {
                let param = typed.pat.require_ident()?.clone();
                for attr in typed.attrs.iter() {
                    if let Some(attr) = attr.path().get_ident() {
                        // FIXME: remove the attr in these cases
                        if attr == "env" {
                            return Ok(ArgKind::Env(param));
                        } else if attr == "tmp" {
                            return Ok(ArgKind::TmpData(param));
                        }
                    }
                }

                return Ok(ArgKind::Passthrough(param));
            }
            FnArg::Receiver(recv) => Ok(ArgKind::Receiver(recv.self_token.clone())),
        }
    }
}

trait PatExt {
    fn require_ident(&self) -> Result<&Ident>;
}

impl PatExt for Pat {
    fn require_ident(&self) -> Result<&Ident> {
        if let Self::Ident(ident) = self {
            Ok(&ident.ident)
        } else {
            Err(Error::new_spanned(
                self,
                "Expected identifier, found pattern",
            ))
        }
    }
}
