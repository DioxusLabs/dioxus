//! Parse anything that has a pattern of < Ident, Bracket >
//! ========================================================
//!
//! Whenever a `name {}` pattern emerges, we need to parse it into an element, a component, or a fragment.
//! This feature must support:
//! - Namepsaced/pathed components
//! - Differentiating between built-in and custom elements

use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    Error, Ident, Result, Token,
};

pub enum AmbiguousElement<const AS: HtmlOrRsx> {
    Element(Element<AS>),
    Component(Component<AS>),
}

impl Parse for AmbiguousElement<AS_RSX> {
    fn parse(input: ParseStream) -> Result<Self> {
        // Try to parse as an absolute path and immediately defer to the componetn
        if input.peek(Token![::]) {
            return input
                .parse::<Component<AS_RSX>>()
                .map(|c| AmbiguousElement::Component(c));
        }

        // If not an absolute path, then parse the ident and check if it's a valid tag

        if let Ok(pat) = input.fork().parse::<syn::Path>() {
            if pat.segments.len() > 1 {
                return input
                    .parse::<Component<AS_RSX>>()
                    .map(|c| AmbiguousElement::Component(c));
            }
        }

        use syn::ext::IdentExt;
        if let Ok(name) = input.fork().call(Ident::parse_any) {
            let name_str = name.to_string();

            let first_char = name_str.chars().next().unwrap();
            if first_char.is_ascii_uppercase() {
                input
                    .parse::<Component<AS_RSX>>()
                    .map(|c| AmbiguousElement::Component(c))
            } else {
                input
                    .parse::<Element<AS_RSX>>()
                    .map(|c| AmbiguousElement::Element(c))
            }
        } else {
            Err(Error::new(input.span(), "Not a valid Html tag"))
        }
    }
}

impl Parse for AmbiguousElement<AS_HTML> {
    fn parse(input: ParseStream) -> Result<Self> {
        // Try to parse as an absolute path and immediately defer to the componetn
        if input.peek(Token![::]) {
            return input
                .parse::<Component<AS_HTML>>()
                .map(|c| AmbiguousElement::Component(c));
        }

        // If not an absolute path, then parse the ident and check if it's a valid tag

        if let Ok(pat) = input.fork().parse::<syn::Path>() {
            if pat.segments.len() > 1 {
                return input
                    .parse::<Component<AS_HTML>>()
                    .map(|c| AmbiguousElement::Component(c));
            }
        }

        use syn::ext::IdentExt;
        if let Ok(name) = input.fork().call(Ident::parse_any) {
            let name_str = name.to_string();

            let first_char = name_str.chars().next().unwrap();
            if first_char.is_ascii_uppercase() {
                input
                    .parse::<Component<AS_HTML>>()
                    .map(AmbiguousElement::Component)
            } else {
                input
                    .parse::<Element<AS_HTML>>()
                    .map(AmbiguousElement::Element)
            }
        } else {
            Err(Error::new(input.span(), "Not a valid Html tag"))
        }
    }
}

impl<const AS: HtmlOrRsx> ToTokens for AmbiguousElement<AS> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            AmbiguousElement::Element(el) => el.to_tokens(tokens),
            AmbiguousElement::Component(comp) => comp.to_tokens(tokens),
        }
    }
}
