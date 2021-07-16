use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token, Expr, LitStr, Result,
};

// ==============================================
// Parse any div {} as a VElement
// ==============================================
pub enum BodyNode {
    Element(AmbiguousElement),
    Text(TextNode),
    RawExpr(Expr),
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        // Supposedly this approach is discouraged due to inability to return proper errors
        // TODO: Rework this to provide more informative errors

        if stream.peek(token::Brace) {
            let content;
            syn::braced!(content in stream);
            return Ok(BodyNode::RawExpr(content.parse::<Expr>()?));
        }

        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse::<TextNode>()?));
        }

        Ok(BodyNode::Element(stream.parse::<AmbiguousElement>()?))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            BodyNode::Element(el) => el.to_tokens(tokens),
            BodyNode::Text(txt) => txt.to_tokens(tokens),
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                 __cx.fragment_from_iter(#exp)
            }),
        }
    }
}

// =======================================
// Parse just plain text
// =======================================
pub struct TextNode(LitStr);

impl Parse for TextNode {
    fn parse(s: ParseStream) -> Result<Self> {
        Ok(Self(s.parse()?))
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // todo: use heuristics to see if we can promote to &static str
        let token_stream = &self.0.to_token_stream();
        tokens.append_all(quote! {
            __cx.text(format_args_f!(#token_stream))
        });
    }
}
