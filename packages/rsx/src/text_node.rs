#[cfg(feature = "hot_reload")]
use dioxus_core::TemplateNode;

use crate::{
    literal::{HotLiteral, HotLiteralType},
    location::DynIdx,
    IfmtInput,
};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{quote, TokenStreamExt};
use syn::Result;
use syn::{
    parse::{Parse, ParseStream},
    LitStr,
};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct TextNode {
    pub input: IfmtInput,
    pub hr_idx: DynIdx,
    pub dyn_idx: DynIdx,
}

impl Parse for TextNode {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            input: input.parse()?,
            hr_idx: DynIdx::default(),
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
            let as_lit = HotLiteral {
                hr_idx: self.hr_idx.clone(),
                value: HotLiteralType::Fmted(txt.clone()),
            };

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
            input: ifmt,
            dyn_idx: Default::default(),
            hr_idx: Default::default(),
        }
    }

    pub fn is_static(&self) -> bool {
        self.input.is_static()
    }

    #[cfg(feature = "hot_reload")]
    pub fn to_template_node(&self) -> TemplateNode {
        use crate::intern;
        match self.is_static() {
            true => {
                let text = self.input.source.clone();
                let text = intern(text.value().as_str());
                TemplateNode::Text { text }
            }
            false => TemplateNode::Dynamic {
                id: self.dyn_idx.get(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrettyUnparse;

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
