use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    token, Expr, LitStr, Result, spanned::Spanned,
};

/*
Parse
-> div {}
-> Component {}
-> component()
-> "text {with_args}"
-> (0..10).map(|f| rsx!("asd")),  // <--- notice the comma - must be a complete expr
*/
#[derive(PartialEq, Eq)]
pub enum BodyNode {
    Element(Element),
    Component(Component),
    Text(LitStr),
    RawExpr(Expr),
}

impl BodyNode {
    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text(_))
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.span(),
            BodyNode::RawExpr(exp) => exp.span(),
        }
    }
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        let body_stream = stream.fork();
        if let Ok(path) = body_stream.parse::<syn::Path>() {
            // this is an Element if path match of:
            // - one ident
            // - followed by `{`
            // - 1st char is lowercase
            //
            // example:
            // div {}
            if let Some(ident) = path.get_ident() {
                if body_stream.peek(token::Brace)
                    && ident
                        .to_string()
                        .chars()
                        .next()
                        .unwrap()
                        .is_ascii_lowercase()
                {
                    return Ok(BodyNode::Element(stream.parse::<Element>()?));
                }
            }

            // Otherwise this should be Component, allowed syntax:
            // - syn::Path
            // - PathArguments can only apper in last segment
            // - followed by `{` or `(`, note `(` cannot be used with one ident
            //
            // example
            // Div {}
            // Div ()
            // ::Div {}
            // crate::Div {}
            // component()
            // ::component {}
            // ::component ()
            // crate::component{}
            // crate::component()
            // Input::<InputProps<'_, i32> {}
            // crate::Input::<InputProps<'_, i32> {}
            if body_stream.peek(token::Brace) || body_stream.peek(token::Paren) {
                Component::validate_component_path(&path)?;

                return Ok(BodyNode::Component(stream.parse()?));
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
        }
    }
}
