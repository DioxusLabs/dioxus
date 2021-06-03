//! Parse components into the VComponent VNode
//! ==========================================
//!
//! This parsing path emerges from [`AmbiguousElement`] which supports validation of the vcomponent format.
//! We can be reasonably sure that whatever enters this parsing path is in the right format.
//! This feature must support
//! - [x] Namespaced components
//! - [x] Fields
//! - [x] Componentbuilder synax
//! - [x] Optional commas
//! - [ ] Children
//! - [ ] Keys
//! - [ ] Properties spreading with with `..` syntax

use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream},
    token, Expr, Ident, Result, Token,
};

pub struct Component {
    // accept any path-like argument
    name: syn::Path,
    body: Vec<ComponentField>,
    _children: Vec<Node>,
}

impl Parse for Component {
    fn parse(s: ParseStream) -> Result<Self> {
        // let name = s.parse::<syn::ExprPath>()?;
        // todo: look into somehow getting the crate/super/etc

        let name = syn::Path::parse_mod_style(s)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in s);

        let mut body: Vec<ComponentField> = Vec::new();
        let _children: Vec<Node> = Vec::new();

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(token::Brace) {
                let inner: ParseBuffer;
                syn::braced!(inner in content);
                if inner.peek(Token![...]) {
                    todo!("Inline props not yet supported");
                }
            }

            body.push(content.parse::<ComponentField>()?);

            // consume comma if it exists
            // we don't actually care if there *are* commas between attrs
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        // todo: add support for children
        let children: Vec<Node> = vec![];

        Ok(Self {
            name,
            body,
            _children: children,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;

        let mut builder = quote! {
            fc_to_builder(#name)
        };

        let mut has_key = None;

        for field in &self.body {
            if field.name.to_string() == "key" {
                has_key = Some(field);
            } else {
                builder.append_all(quote! {#field});
            }
        }

        builder.append_all(quote! {
            .build()
        });

        let key_token = match has_key {
            Some(field) => {
                let inners = field.content.to_token_stream();
                quote! {
                    Some(#inners)
                }
            }
            None => quote! {None},
        };

        let _toks = tokens.append_all(quote! {
            dioxus::builder::virtual_child(__ctx, #name, #builder, #key_token)
        });
    }
}

// the struct's fields info
pub struct ComponentField {
    name: Ident,
    content: Expr,
}

impl Parse for ComponentField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let content = input.parse()?;

        Ok(Self { name, content })
    }
}

impl ToTokens for ComponentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! {
            .#name(#content)
        })
    }
}
