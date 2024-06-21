use location::DynIdx;
use proc_macro2::TokenStream as TokenStream2;
use syn::LitStr;

use self::literal::{HotLiteral, RsxLiteral};
use super::*;

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
            let as_lit = RsxLiteral {
                hr_idx: self.hr_idx.clone(),
                raw: syn::Lit::Str(txt.source.as_ref().unwrap().clone()),
                value: HotLiteral::Fmted(txt.clone()),
            };

            tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new( #as_lit ))
            })
        }
    }
}

impl TextNode {
    pub fn from_text(input: &str) -> Self {
        Self {
            input: IfmtInput::new_static(input),
            hr_idx: DynIdx::default(),
            dyn_idx: DynIdx::default(),
        }
    }

    pub fn from_listr(input: LitStr) -> Self {
        Self {
            input: IfmtInput::new_litstr(input),
            hr_idx: DynIdx::default(),
            dyn_idx: DynIdx::default(),
        }
    }

    pub fn is_static(&self) -> bool {
        self.input.is_static()
    }

    pub fn to_template_node(&self) -> TemplateNode {
        match self.is_static() {
            true => {
                let text = self.input.source.as_ref().unwrap();
                let text = intern(text.value().as_str());
                TemplateNode::Text { text }
            }
            false => TemplateNode::DynamicText {
                id: self.dyn_idx.get(),
            },
        }
    }
}

#[test]
fn parses() {
    let input = syn::parse2::<TextNode>(quote! { "hello world" }).unwrap();
    assert_eq!(input.input.source.unwrap().value(), "hello world");
}

#[test]
fn to_tokens_with_hr() {
    let lit = syn::parse2::<TextNode>(quote! { "hi {world1} {world2} {world3}" }).unwrap();
    println!("{}", lit.to_token_stream().pretty_unparse());
}
