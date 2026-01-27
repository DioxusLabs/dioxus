use crate::{HotReloadFormattedSegment, IfmtInput, literal::HotLiteral, location::DynIdx};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{TokenStreamExt, quote};
use syn::Result;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct TextNode {
    pub input: HotReloadFormattedSegment,
    pub dyn_idx: DynIdx,
}

impl Parse for TextNode {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            input: input.parse()?,
            dyn_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let txt = &self.input;

        if txt.is_static() {
            tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
            })
        } else {
            // todo:
            // Use the RsxLiteral implementation to spit out a hotreloadable variant of this string
            // This is not super efficient since we're doing a bit of cloning
            let as_lit = HotLiteral::Fmted(txt.clone());

            tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new( #as_lit ))
            })
        }
    }
}

impl TextNode {
    pub fn from_text(text: &str) -> Self {
        let ifmt = IfmtInput {
            source: LitStr::new(text, Span::call_site()),
            segments: vec![],
        };
        Self {
            input: ifmt.into(),
            dyn_idx: Default::default(),
        }
    }

    pub fn is_static(&self) -> bool {
        self.input.is_static()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prettier_please::PrettyUnparse;

    #[test]
    fn parses() {
        let input = syn::parse2::<TextNode>(quote! { "hello world" }).unwrap();
        assert_eq!(input.input.source.value(), "hello world");
    }

    #[test]
    fn to_tokens_with_hr() {
        let lit = syn::parse2::<TextNode>(quote! { "hi {world1} {world2} {world3}" }).unwrap();
        println!("{}", lit.to_token_stream().pretty_unparse());
    }

    #[test]
    fn raw_str() {
        let input = syn::parse2::<TextNode>(quote! { r#"hello world"# }).unwrap();
        println!("{}", input.input.source.to_token_stream());
        assert_eq!(input.input.source.value(), "hello world");
    }
}
