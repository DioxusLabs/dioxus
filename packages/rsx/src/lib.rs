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
mod node;

pub mod pretty;

// Re-export the namespaces into each other
pub use component::*;
pub use element::*;
pub use node::*;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result, Token,
};

pub struct CallBody {
    pub custom_context: Option<Ident>,
    pub roots: Vec<BodyNode>,
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let custom_context = if input.peek(Ident) && input.peek2(Token![,]) {
            let name = input.parse::<Ident>()?;
            input.parse::<Token![,]>()?;

            Some(name)
        } else {
            None
        };

        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = input.parse::<BodyNode>()?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        Ok(Self {
            custom_context,
            roots,
        })
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

        match &self.custom_context {
            // The `in cx` pattern allows directly rendering
            Some(ident) => out_tokens.append_all(quote! {
                #ident.render(LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};
                    #inner
                }))
            }),

            // Otherwise we just build the LazyNode wrapper
            None => out_tokens.append_all(quote! {
                LazyNodes::new(move |__cx: NodeFactory| -> VNode {
                    use dioxus_elements::{GlobalAttributes, SvgAttributes};
                    #inner
                })
            }),
        };
    }
}
