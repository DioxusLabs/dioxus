use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token, Attribute, Expr, LitStr, Result, Token,
};

/*
Parse
-> div {}
-> Component {}
-> component()
-> "text {with_args}"
-> (0..10).map(|f| rsx!("asd")),  // <--- notice the comma - must be a complete expr
*/
pub enum BodyNode {
    Element(Element),
    Component(Component),
    Text(LitStr),
    RawExpr(Expr),
    Meta(Attribute),
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        // div {} -> el
        // Div {} -> comp
        if stream.peek(syn::Ident) && stream.peek2(token::Brace) {
            if stream
                .fork()
                .parse::<Ident>()?
                .to_string()
                .chars()
                .next()
                .unwrap()
                .is_ascii_uppercase()
            {
                return Ok(BodyNode::Component(stream.parse()?));
            } else {
                return Ok(BodyNode::Element(stream.parse::<Element>()?));
            }
        }

        // component() -> comp
        // ::component {} -> comp
        // ::component () -> comp
        if (stream.peek(syn::Ident) && stream.peek2(token::Paren))
            || (stream.peek(Token![::]))
            || (stream.peek(Token![:]) && stream.peek2(Token![:]))
        {
            return Ok(BodyNode::Component(stream.parse::<Component>()?));
        }

        // crate::component{} -> comp
        // crate::component() -> comp
        if let Ok(pat) = stream.fork().parse::<syn::Path>() {
            if pat.segments.len() > 1 {
                return Ok(BodyNode::Component(stream.parse::<Component>()?));
            }
        }

        Ok(BodyNode::RawExpr(stream.parse::<Expr>()?))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            BodyNode::Element(el) => el.to_tokens(tokens),
            BodyNode::Component(comp) => comp.to_tokens(tokens),
            BodyNode::Text(txt) => tokens.append_all(quote! {
                __cx.text(format_args_f!(#txt))
            }),
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                 __cx.fragment_from_iter(#exp)
            }),
            BodyNode::Meta(_) => {}
        }
    }
}
