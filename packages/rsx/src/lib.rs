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

#[macro_use]
mod errors;

mod component;
mod element;
mod ifmt;
mod node;

// Re-export the namespaces into each other
pub use component::*;
pub use element::*;
pub use ifmt::*;
pub use node::*;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

pub struct CallBody {
    pub roots: Vec<BodyNode>,
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        Ok(Self { roots })
    }
}

/// Serialize the same way, regardless of flavor
impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let inner = if self.roots.len() == 1 {
            let inner = &self.roots[0];
            quote! { #inner }
        } else {
            let childs = &self.roots;
            quote! { __cx.fragment_root([ #(#childs),* ]) }
        };

        // Otherwise we just build the LazyNode wrapper
        out_tokens.append_all(quote! {
            LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                use dioxus_elements::{GlobalAttributes, SvgAttributes};
                #inner
            })
        })
    }
}
