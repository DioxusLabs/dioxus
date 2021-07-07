//! Parse the root tokens in the rsx!{} macro
//! =========================================
//!
//! This parsing path emerges directly from the macro call, with `RsxRender` being the primary entrance into parsing.
//! This feature must support:
//! - [x] Optionally rendering if the `in XYZ` pattern is present
//! - [x] Fragments as top-level element (through ambiguous)
//! - [x] Components as top-level element (through ambiguous)
//! - [x] Tags as top-level elements (through ambiguous)
//! - [x] Good errors if parsing fails
//!
//! Any errors in using rsx! will likely occur when people start using it, so the first errors must be really helpful.

mod ambiguous;
mod component;
mod element;
mod fragment;
mod node;

// Re-export the namespaces into each other
pub use ambiguous::*;
pub use component::*;
pub use element::*;
pub use fragment::*;
pub use node::*;

use crate::util::is_valid_tag;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Error, Ident, LitStr, Result, Token,
};

pub struct RsxRender {
    custom_context: Option<Ident>,
    roots: Vec<AmbiguousElement>,
}

impl Parse for RsxRender {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            return input.parse::<LitStr>()?.parse::<RsxRender>();
        }

        // try to parse the first ident and comma
        let custom_context =
            if input.peek(Token![in]) && input.peek2(Ident) && input.peek3(Token![,]) {
                let _ = input.parse::<Token![in]>()?;
                let name = input.parse::<Ident>()?;
                if is_valid_tag(&name.to_string()) {
                    return Err(Error::new(
                        input.span(),
                        "Custom context cannot be an html element name",
                    ));
                } else {
                    input.parse::<Token![,]>().unwrap();
                    Some(name)
                }
            } else {
                None
            };

        let mut roots = Vec::new();
        while !input.is_empty() {
            roots.push(input.parse::<AmbiguousElement>()?);
        }

        Ok(Self {
            roots,
            custom_context,
        })
    }
}

impl ToTokens for RsxRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let inner = if self.roots.len() == 1 {
            let inner = &self.roots[0];
            quote! {#inner}
        } else {
            let childs = &self.roots;
            quote! { __cx.fragment_from_iter(&[ #(#childs),* ]) }
        };

        match &self.custom_context {
            // The `in cx` pattern allows directly rendering
            Some(ident) => out_tokens.append_all(quote! {
                #ident.render(dioxus::prelude::LazyNodes::new(move |__cx: &NodeFactory|{
                    #inner
                }))
            }),
            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                dioxus::prelude::LazyNodes::new(move |__cx: &NodeFactory|{
                    #inner
                 })
            }),
        };
    }
}
