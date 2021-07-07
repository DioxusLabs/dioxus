use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse, ParseBuffer, ParseStream},
    token, Error, Expr, ExprClosure, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
pub struct Element {
    name: Ident,
    key: Option<AttrType>,
    attributes: Vec<ElementAttr>,
    listeners: Vec<ElementAttr>,
    children: Vec<Node>,
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        // let name = &self.name.to_string();

        tokens.append_all(quote! {
            __cx.element(#name)
        });

        // Add attributes
        // TODO: conver to the "attrs" syntax for compile-time known sizes
        if self.attributes.len() > 0 {
            let attr = &self.attributes;
            tokens.append_all(quote! {
                .attributes([ #(#attr),* ])
            })
        }

        if self.children.len() > 0 {
            let childs = &self.children;
            tokens.append_all(quote! {
                .children([ #(#childs),* ])
            });
        }

        if self.listeners.len() > 0 {
            let listeners = &self.listeners;
            tokens.append_all(quote! {
                .listeners([ #(#listeners),* ])
            });
        }

        tokens.append_all(quote! {
            .finish()
        });
    }
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = Ident::parse(stream)?;

        // TODO: remove this in favor of the compile-time validation system
        if !crate::util::is_valid_tag(&name.to_string()) {
            return Err(Error::new(name.span(), "Not a valid Html tag"));
        }

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttr> = vec![];
        let mut listeners: Vec<ElementAttr> = vec![];
        let mut children: Vec<Node> = vec![];
        let mut key = None;

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                parse_element_body(&content, &mut attributes, &mut listeners, &mut key)?;
            } else {
                children.push(content.parse::<Node>()?);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas after elements/text
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            key,
            name,
            attributes,
            children,
            listeners,
        })
    }
}

/// =======================================
/// Parse a VElement's Attributes
/// =======================================
struct ElementAttr {
    name: Ident,
    value: AttrType,
    namespace: Option<String>,
}

enum AttrType {
    BumpText(LitStr),
    FieldTokens(Expr),
    EventTokens(Expr),
    Event(ExprClosure),
}

// We parse attributes and dump them into the attribute vec
// This is because some tags might be namespaced (IE style)
// These dedicated tags produce multiple name-spaced attributes
fn parse_element_body(
    stream: ParseStream,
    attrs: &mut Vec<ElementAttr>,
    listeners: &mut Vec<ElementAttr>,
    key: &mut Option<AttrType>,
) -> Result<()> {
    let mut name = Ident::parse_any(stream)?;
    let name_str = name.to_string();
    stream.parse::<Token![:]>()?;

    // Return early if the field is a listener
    if name_str.starts_with("on") {
        // remove the "on" bit
        // name = Ident::new(&name_str.trim_start_matches("on"), name.span());

        let ty = if stream.peek(token::Brace) {
            let content;
            syn::braced!(content in stream);

            // Try to parse directly as a closure
            let fork = content.fork();
            if let Ok(event) = fork.parse::<ExprClosure>() {
                content.advance_to(&fork);
                AttrType::Event(event)
            } else {
                AttrType::EventTokens(content.parse()?)
            }
        } else {
            AttrType::Event(stream.parse()?)
        };
        listeners.push(ElementAttr {
            name,
            value: ty,
            namespace: None,
        });
        return Ok(());
    }

    let ty: AttrType = match name_str.as_str() {
        // short circuit early if style is using the special syntax
        "style" if stream.peek(token::Brace) => {
            let inner;
            syn::braced!(inner in stream);

            while !inner.is_empty() {
                let name = Ident::parse_any(&inner)?;
                inner.parse::<Token![:]>()?;
                let ty = if inner.peek(LitStr) {
                    let rawtext = inner.parse::<LitStr>().unwrap();
                    AttrType::BumpText(rawtext)
                } else {
                    let toks = inner.parse::<Expr>()?;
                    AttrType::FieldTokens(toks)
                };
                if inner.peek(Token![,]) {
                    let _ = inner.parse::<Token![,]>();
                }
                attrs.push(ElementAttr {
                    name,
                    value: ty,
                    namespace: Some("style".to_string()),
                });
            }

            return Ok(());
        }
        "key" => {
            *key = Some(AttrType::BumpText(stream.parse::<LitStr>()?));
            return Ok(());
        }
        "classes" => {
            todo!("custom class lsit not supported")
        }
        "namespace" => {
            todo!("custom namespace not supported")
        }
        "ref" => {
            todo!("NodeRefs are currently not supported! This is currently a reserved keyword.")
        }

        // Fall through
        _ => {
            if stream.peek(LitStr) {
                let rawtext = stream.parse::<LitStr>().unwrap();
                AttrType::BumpText(rawtext)
            } else {
                let toks = stream.parse::<Expr>()?;
                AttrType::FieldTokens(toks)
            }
        }
    };

    // consume comma if it exists
    // we don't actually care if there *are* commas between attrs
    if stream.peek(Token![,]) {
        let _ = stream.parse::<Token![,]>();
    }

    attrs.push(ElementAttr {
        name,
        value: ty,
        namespace: None,
    });
    Ok(())
}

impl ToTokens for ElementAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.name.to_string();
        let nameident = &self.name;

        let namespace = match &self.namespace {
            Some(t) => quote! { Some(#t) },
            None => quote! { None },
        };

        match &self.value {
            AttrType::BumpText(value) => tokens.append_all(quote! {
                __cx.attr(#name, format_args_f!(#value), #namespace)
            }),

            AttrType::FieldTokens(exp) => tokens.append_all(quote! {
                __cx.attr(#name, #exp, #namespace)
            }),

            // todo: move event handlers on to the elements or onto the nodefactory
            AttrType::Event(event) => tokens.append_all(quote! {
                dioxus::events::on::#nameident(__cx, #event)
            }),

            AttrType::EventTokens(event) => tokens.append_all(quote! {
                dioxus::events::on::#nameident(__cx, #event)
            }),
        }
    }
}
