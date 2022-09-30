use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse::Parse, LitStr};

use crate::whitespace::Whitespace;

#[derive(Debug, PartialEq, Eq)]
pub struct TextNode {
    pub ws: Whitespace,
    pub text: LitStr,
}

impl TextNode {
    pub fn span(&self) -> Span {
        self.text.span()
    }
}

impl Parse for TextNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ws: Whitespace::default(),
            text: input.parse()?,
        })
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let text = &self.text;
        tokens.append_all(quote! { #text });
    }
}
