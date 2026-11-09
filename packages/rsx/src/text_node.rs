use crate::{HotReloadFormattedSegment, IfmtInput, literal::HotLiteral};
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
}

impl Parse for TextNode {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            input: input.parse()?,
        })
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let txt = &self.input;

        if txt.is_static() {
            tokens.append_all(quote! {
                dioxus_vdom::DynamicNode::Text(dioxus_vdom::VText::new(#txt.to_string()))
            })
        } else {
            // todo:
            // Use the RsxLiteral implementation to spit out a hotreloadable variant of this string
            // This is not super efficient since we're doing a bit of cloning
            let as_lit = HotLiteral::Fmted(txt.clone());

            tokens.append_all(quote! {
                dioxus_vdom::DynamicNode::Text(dioxus_vdom::VText::new( #as_lit ))
            })
        }
    }
}

impl TextNode {
    pub fn from_text(text: &str) -> Self {
        // Text coming from raw sources (like HTML) is always literal, so any `{`/`}` must be
        // escaped to avoid being parsed as format-string interpolation when the generated `rsx!`
        // source is compiled.
        let source = LitStr::new(&IfmtInput::escape_literal_braces(text), Span::call_site());
        let ifmt = IfmtInput::new_litstr(source)
            .expect("escaping the braces should always produce a valid format string");
        Self { input: ifmt.into() }
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
