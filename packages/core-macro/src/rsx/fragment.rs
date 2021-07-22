//! Parse `Fragments` into the Fragment VNode
//! ==========================================
//!
//! This parsing path emerges from [`AmbiguousElement`] which supports validation of the Fragment format.
//! We can be reasonably sure that whatever enters this parsing path is in the right format.
//! This feature must support:
//! - [x] Optional commas
//! - [ ] Children
//! - [ ] Keys

use syn::parse::ParseBuffer;

use super::{AmbiguousElement, AS_HTML, AS_RSX, HTML_OR_RSX};

use {
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::{quote, ToTokens, TokenStreamExt},
    syn::{
        ext::IdentExt,
        parse::{Parse, ParseStream},
        token, Error, Expr, ExprClosure, Ident, LitBool, LitStr, Path, Result, Token,
    },
};

pub struct Fragment<const AS: HTML_OR_RSX> {
    children: Vec<AmbiguousElement<AS>>,
}

impl Parse for Fragment<AS_RSX> {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Ident>()?;

        let children = Vec::new();

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in input);
        while !content.is_empty() {
            content.parse::<AmbiguousElement<AS_RSX>>()?;

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }
        Ok(Self { children })
    }
}

impl Parse for Fragment<AS_HTML> {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Ident>()?;

        let children = Vec::new();

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in input);
        while !content.is_empty() {
            content.parse::<AmbiguousElement<AS_HTML>>()?;

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }
        Ok(Self { children })
    }
}

impl<const AS: HTML_OR_RSX> ToTokens for Fragment<AS> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let childs = &self.children;
        let children = quote! {
            ChildrenList::new(__cx)
                #( .add_child(#childs) )*
                .finish()
        };
        tokens.append_all(quote! {
            // #key_token,
            dioxus::builder::vfragment(
                __cx,
                None,
                #children
            )
        })
    }
}
