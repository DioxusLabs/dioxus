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
//!
//! # Completions
//! Rust analyzer completes macros by looking at the expansion of the macro and trying to match the start of identifiers in the macro to identifiers in the current scope
//!
//! Eg, if a macro expands to this:
//! ```rust, ignore
//! struct MyStruct;
//!
//! // macro expansion
//! My
//! ```
//! Then the analyzer will try to match the start of the identifier "My" to an identifier in the current scope (MyStruct in this case).
//!
//! In dioxus, our macros expand to the completions module if we know the identifier is incomplete:
//! ```rust, ignore
//! // In the root of the macro, identifiers must be elements
//! // rsx! { di }
//! dioxus_elements::elements::di
//!
//! // Before the first child element, every following identifier is either an attribute or an element
//! // rsx! { div { ta } }
//! // Isolate completions scope
//! mod completions__ {
//!     // import both the attributes and elements this could complete to
//!     use dioxus_elements::elements::div::*;
//!     use dioxus_elements::elements::*;
//!     fn complete() {
//!         ta;
//!     }
//! }
//!
//! // After the first child element, every following identifier is another element
//! // rsx! { div { attribute: value, child {} di } }
//! dioxus_elements::elements::di
//! ```

#[macro_use]
mod errors;
mod attribute;
mod component;
mod element;
mod ifmt;
mod location;
mod node;
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
use dioxus_core::{Template, TemplateAttribute, TemplateNode};
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

        quote! { { #body } }
    }

    /// This will try to create a new template from the current body and the previous body. This will return None if the
    /// rsx has some dynamic part that has changed.
    ///
    /// The previous_location is the location of the previous template at the time the template was originally compiled.
    /// It's up to you the implementor to trace the template location back to the original source code. Generally you
    /// can simply just match the location from the syn::File type to the template map living in the renderer.
    ///
    /// When you implement hotreloading, you're likely just going to parse the source code into the Syn::File type, which
    /// should make retrieving the template location easy.
    ///
    /// ## Note:
    ///
    ///  - This function intentionally leaks memory to create a static template.
    ///  - Keeping the template static allows us to simplify the core of dioxus and leaking memory in dev mode is less of an issue.
    ///
    /// ## Longer note about sub templates:
    ///
    ///    Sub templates when expanded in rustc use the same file/lin/col information as the parent template. This can
    ///    be annoying when you're trying to get a location for a sub template and it's pretending that it's its parent.
    ///    The new implementation of this aggregates all subtemplates into the TemplateRenderer and then assigns them
    ///    unique IDs based on the byte index of the template, working around this issue.
    ///
    /// ## TODO:
    ///
    ///    A longer term goal would be to provide some sort of diagnostics to the user as to why the template was not
    ///    updated, giving them an option to revert to the previous template as to not require a full rebuild.
    #[cfg(feature = "hot_reload")]
    pub fn update_template<Ctx: HotReloadingContext>(
        &self,
        old: Option<CallBody>,
        location: &'static str,
    ) -> Option<Template> {
        // Create a context that will be used to update the template
        let mut context = DynamicContext::new_with_old(old);

        // Force the template node to generate us TemplateNodes, and fill in the location information
        let roots = context.populate_by_updating::<Ctx>(&self.roots)?;

        // We've received the dioxus-core TemplateNodess, and need to assemble them into a Template
        // We could just use them directly, but we want to intern them to do our best to avoid
        // egregious memory leaks. We're sitll leaking memory, but at least we can blame it on
        // the `Intern` crate and not just the fact that we call Box::leak.
        //
        // We should also note that order of these nodes could be all scrambeled
        Some(Template {
            name: location,
            roots: intern(roots.as_slice()),
            node_paths: intern(
                context
                    .node_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            attr_paths: intern(
                context
                    .attr_paths
                    .into_iter()
                    .map(|path| intern(path.as_slice()))
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        })
    }

    /// Parse a stream into a CallBody. Return all error immediately instead of trying to partially expand the macro
    ///
    /// This should be preferred over `parse` if you are outside of a macro
    pub fn parse_strict(input: ParseStream) -> Result<Self> {
        Self::parse_with_options(input, false)
    }

    fn parse_with_options(input: ParseStream, partial_completions: bool) -> Result<Self> {
        let mut roots = Vec::new();

        while !input.is_empty() {
            let node = BodyNode::parse_with_options(input, partial_completions)?;

            if input.peek(Token![,]) {
                let _ = input.parse::<Token![,]>();
            }

            roots.push(node);
        }

        Ok(CallBody { roots })
    }
}

impl Parse for CallBody {
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_with_options(input, true)
    }
}

impl ToTokens for CallBody {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        // Empty templates just are placeholders for "none"
        match self.roots.is_empty() {
            true => out_tokens
                .append_all(quote! { dioxus_core::prelude::Element::Ok(dioxus_core::prelude::VNode::placeholder()) }),
            false => {
                let body = TemplateRenderer::as_tokens(&self.roots, None);
                out_tokens.append_all(quote! { { #body } })
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
