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
    children: Vec<Node>,
    manual_props: Option<Expr>,
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        // let name = s.parse::<syn::ExprPath>()?;
        // todo: look into somehow getting the crate/super/etc

        let name = syn::Path::parse_mod_style(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut body: Vec<ComponentField> = Vec::new();
        let mut children: Vec<Node> = Vec::new();
        let mut manual_props = None;

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                manual_props = Some(content.parse::<Expr>()?);
            } else if content.peek(Ident) && content.peek2(Token![:]) {
                body.push(content.parse::<ComponentField>()?);
            } else {
                children.push(content.parse::<Node>()?);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas between attrs
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            name,
            body,
            children,
            manual_props,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;

        let using_manual_override = self.manual_props.is_some();

        let mut builder = {
            match &self.manual_props {
                Some(manual_props) => quote! { #manual_props },
                None => quote! { fc_to_builder(#name) },
            }
        };

        let mut has_key = None;

        for field in &self.body {
            if field.name.to_string() == "key" {
                has_key = Some(field);
            } else {
                match using_manual_override {
                    true => panic!("Currently we don't support manual props and prop fields. Choose either manual props or prop fields"),
                    false => builder.append_all(quote! {#field}),
                }
            }
        }

        if !using_manual_override {
            builder.append_all(quote! {
                .build()
            });
        }

        let key_token = match has_key {
            Some(field) => {
                let inners = field.content.to_token_stream();
                quote! {
                    Some(#inners)
                }
            }
            None => quote! {None},
        };

        let childs = &self.children;
        let children = quote! {
            ChildrenList::new(__ctx)
                #( .add_child(#childs) )*
                .finish()
        };

        tokens.append_all(quote! {
            dioxus::builder::virtual_child(
                __ctx,
                #name,
                #builder,
                #key_token,
                #children
            )
        })
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
