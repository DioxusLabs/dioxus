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
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Error, Ident, LitStr, Result, Token,
};

pub struct RsxRender {
    custom_context: Option<Ident>,
    root: AmbiguousElement,
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

        let root = { input.parse::<AmbiguousElement>() }?;
        if !input.is_empty() {
            return Err(Error::new(
                input.span(),
                "Currently only one element is allowed per component. Try wrapping your list of components in a `Fragment` tag",
            ));
        }

        Ok(Self {
            root,
            custom_context,
        })
    }
}

impl ToTokens for RsxRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let inner = &self.root;
        let output = match &self.custom_context {
            // The `in ctx` pattern allows directly rendering
            Some(ident) => {
                quote! {
                    #ident.render(dioxus::prelude::LazyNodes::new(move |__ctx|{
                        let bump = &__ctx.bump();
                        #inner
                    }))
                }
            }
            // Otherwise we just build the LazyNode wrapper
            None => {
                quote! {
                    dioxus::prelude::LazyNodes::new(move |__ctx|{
                        let bump = &__ctx.bump();
                        #inner
                     })
                }
            }
        };
        output.to_tokens(out_tokens)
    }
}
