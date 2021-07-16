//! Html body
//! -------
//!
//!
//! Since both HTML and RSX serialize to the same node structure, the HTML parser uses the same types as RSX,
//! but has a different Parse implementation.

use crate::rsx::*;
use quote::ToTokens;
use syn::parse::Parse;

pub struct HtmlBody(RsxBody);

impl Parse for HtmlBody {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}
impl ToTokens for HtmlBody {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens)
    }
}

pub struct HtmlNode(BodyNode);
pub struct HtmlAmbigiousElement(AmbiguousElement);
pub struct HtmlComponent(Component);
