#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

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
mod attribute;
mod build_template;
mod component;
mod element;
mod ifmt;
mod location;
mod node;
pub mod tracked;
mod util;

pub(crate) mod context;
pub(crate) mod renderer;
mod sub_templates;

// Re-export the namespaces into each other
pub use attribute::*;
pub use component::*;
pub use context::DynamicContext;
pub use element::*;
pub use ifmt::*;
pub use node::*;

#[cfg(feature = "hot_reload")]
pub mod hot_reload;

#[cfg(feature = "hot_reload")]
use dioxus_core::{TemplateAttribute, TemplateNode};
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;
#[cfg(feature = "hot_reload")]
use internment::Intern;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use renderer::TemplateRenderer;
use std::{fmt::Debug, hash::Hash};
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

/// The Callbody is the contents of the rsx! macro
///
/// It is a list of BodyNodes, which are the different parts of the template.
/// The Callbody contains no information about how the template will be rendered, only information about the parsed tokens.
///
/// Every callbody should be valid, so you can use it to build a template.
/// To generate the code used to render the template, use the ToTokens impl on the Callbody, or with the `render_with_location` method.
#[derive(Default, Debug)]
pub struct CallBody {
    pub roots: Vec<BodyNode>,
}

impl CallBody {
    /// Render the template with a manually set file location. This should be used when multiple rsx! calls are used in the same macro
    pub fn render_with_location(&self, location: String) -> TokenStream2 {
        // Empty templates just are placeholders for "none"
        if self.roots.is_empty() {
            return quote! { None };
        }

        let body = TemplateRenderer::as_tokens(&self.roots, Some(location));

        quote! { Some({ #body }) }
    }
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

        Ok(CallBody { roots })
    }
}

impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        // Empty templates just are placeholders for "none"
        match self.roots.is_empty() {
            true => out_tokens.append_all(quote! { None }),
            false => {
                let body = TemplateRenderer::as_tokens(&self.roots, None);
                out_tokens.append_all(quote! { Some({ #body }) })
            }
        }
    }
}

#[cfg(feature = "hot_reload")]
// interns a object into a static object, resusing the value if it already exists
pub(crate) fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(
    s: impl Into<Intern<T>>,
) -> &'static T {
    s.into().as_ref()
}
