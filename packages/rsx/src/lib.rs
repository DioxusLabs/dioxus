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
mod component;
mod element;
mod ifmt;
mod node;

pub(crate) mod context;
pub(crate) mod mapping;
pub(crate) mod renderer;

use std::{fmt::Debug, hash::Hash};

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
use dioxus_core::{Template, TemplateAttribute, TemplateNode};
#[cfg(feature = "hot_reload")]
pub use hot_reload::HotReloadingContext;
#[cfg(feature = "hot_reload")]
use internment::Intern;

// imports
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use renderer::TemplateRenderer;
use syn::{
    parse::{Parse, ParseStream},
    Result, Token,
};

#[cfg(feature = "hot_reload")]
// interns a object into a static object, resusing the value if it already exists
fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(s: impl Into<Intern<T>>) -> &'static T {
    s.into().as_ref()
}

/// Fundametnally, every CallBody is a template
#[derive(Default, Debug)]
pub struct CallBody {
    pub roots: Vec<BodyNode>,
}

impl CallBody {
    /// Render the template with a manually set file location. This should be used when multiple rsx! calls are used in the same macro
    pub fn render_with_location(&self, location: String) -> TokenStream2 {
        let body = TemplateRenderer::new(&self.roots, Some(location));

        // Empty templates just are placeholders for "none"
        if self.roots.is_empty() {
            return quote! { None };
        }

        quote! {
            Some({ #body })
        }
    }

    #[cfg(feature = "hot_reload")]
    /// This will try to create a new template from the current body and the previous body. This will return None if the rsx has some dynamic part that has changed.
    /// This function intentionally leaks memory to create a static template.
    /// Keeping the template static allows us to simplify the core of dioxus and leaking memory in dev mode is less of an issue.
    /// the previous_location is the location of the previous template at the time the template was originally compiled.
    pub fn update_template<Ctx: HotReloadingContext>(
        &self,
        template: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        let mut renderer = TemplateRenderer::new(&self.roots, None);

        renderer.update_template::<Ctx>(template, location)
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

        Ok(Self { roots })
    }
}

impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let body = TemplateRenderer::new(&self.roots, None);

        // Empty templates just are placeholders for "none"
        if self.roots.is_empty() {
            return out_tokens.append_all(quote! { None });
        }

        out_tokens.append_all(quote! {
            Some({ #body })
        })
    }
}
