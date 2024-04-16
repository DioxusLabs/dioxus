//! A collection of body nodes that are parsed as a template
//!
//! When this has `ToTokens`` called on it, it will generate a template, not a list of nodes
//!
//!

use crate::BodyNode;
use dioxus_core::TemplateNode;
use quote::ToTokens;
use syn::{parse::Parse, token};

type NodePath = Vec<u8>;
type AttributePath = Vec<u8>;

/// The body of some rsx, potentially including the brace token
///
/// {
///     div { "hi" }
///     div { "hi" }
/// }
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Body {
    children: Vec<BodyNode>,
    template: Vec<TemplateNode>,
    node_paths: Vec<NodePath>,
    attr_paths: Vec<AttributePath>,
}

impl Parse for Body {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

impl ToTokens for Body {
    /// construct a template, not a list of nodes
    ///
    /// This will use the location data
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        todo!()
    }
}
