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
pub struct Element<const AS: HTML_OR_RSX> {
    name: Ident,
    key: Option<LitStr>,
    attributes: Vec<ElementAttr<AS>>,
    listeners: Vec<ElementAttr<AS>>,
    children: Vec<BodyNode<AS>>,
    is_static: bool,
}

impl Parse for Element<AS_RSX> {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = Ident::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttr<AS_RSX>> = vec![];
        let mut listeners: Vec<ElementAttr<AS_RSX>> = vec![];
        let mut children: Vec<BodyNode<AS_RSX>> = vec![];
        let mut key = None;
        let mut el_ref = None;

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                parse_rsx_element_field(
                    &content,
                    &mut attributes,
                    &mut listeners,
                    &mut key,
                    &mut el_ref,
                    name.clone(),
                )?;
            } else {
                children.push(content.parse::<BodyNode<AS_RSX>>()?);
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
            is_static: false,
        })
    }
}

impl Parse for Element<AS_HTML> {
    fn parse(stream: ParseStream) -> Result<Self> {
        let l_tok = stream.parse::<Token![<]>()?;
        let el_name = Ident::parse(stream)?;

        // parse the guts
        // let content: ParseBuffer;
        // syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttr<AS_HTML>> = vec![];
        let mut listeners: Vec<ElementAttr<AS_HTML>> = vec![];
        let mut children: Vec<BodyNode<AS_HTML>> = vec![];
        let mut key = None;

        // loop {
        //     if stream.peek(Token![>]) {
        //         break;
        //     } else {
        //     }
        // }
        while !stream.peek(Token![>]) {
            // self-closing
            if stream.peek(Token![/]) {
                stream.parse::<Token![/]>()?;
                stream.parse::<Token![>]>()?;

                return Ok(Self {
                    name: el_name,
                    key: None,
                    attributes,
                    is_static: false,
                    listeners,
                    children,
                });
            }

            let name = Ident::parse_any(stream)?;
            let name_str = name.to_string();
            stream.parse::<Token![=]>()?;
            if name_str.starts_with("on") {
                let inner;
                syn::braced!(inner in stream);
                let toks = inner.parse::<Expr>()?;
                let ty = AttrType::EventTokens(toks);
                listeners.push(ElementAttr {
                    element_name: el_name.clone(),
                    name,
                    value: ty,
                    namespace: None,
                })
            } else {
                match name_str.as_str() {
                    "style" => {}
                    "key" => {}
                    "classes" | "namespace" | "ref" | _ => {
                        let ty = if stream.peek(LitStr) {
                            let rawtext = stream.parse::<LitStr>().unwrap();
                            AttrType::BumpText(rawtext)
                        } else {
                            // like JSX, we expect raw expressions
                            let inner;
                            syn::braced!(inner in stream);
                            let toks = inner.parse::<Expr>()?;
                            AttrType::FieldTokens(toks)
                        };
                        attributes.push(ElementAttr {
                            element_name: el_name.clone(),
                            name,
                            value: ty,
                            namespace: None,
                        })
                    }
                }
            };
        }
        stream.parse::<Token![>]>()?;

        // closing element
        stream.parse::<Token![<]>()?;
        stream.parse::<Token![/]>()?;
        let close = Ident::parse_any(stream)?;
        if close.to_string() != el_name.to_string() {
            return Err(Error::new_spanned(
                close,
                "closing element does not match opening",
            ));
        }
        stream.parse::<Token![>]>()?;
        // 'parsing: loop {
        //     // if stream.peek(Token![>]) {}

        //     // // [1] Break if empty
        //     // if content.is_empty() {
        //     //     break 'parsing;
        //     // }

        //     if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
        //         parse_element_body(
        //             &content,
        //             &mut attributes,
        //             &mut listeners,
        //             &mut key,
        //             name.clone(),
        //         )?;
        //     } else {
        //         children.push(stream.parse::<BodyNode<AS_HTML>>()?);
        //     }
        // }

        Ok(Self {
            key,
            name: el_name,
            attributes,
            children,
            listeners,
            is_static: false,
        })
    }
}

impl<const AS: HTML_OR_RSX> ToTokens for Element<AS> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let attr = &self.attributes;
        let childs = &self.children;
        let listeners = &self.listeners;
        let key = match &self.key {
            Some(ty) => quote! { Some(format_args_f!(#ty)) },
            None => quote! { None },
        };

        tokens.append_all(quote! {
            __cx.element(
                dioxus_elements::#name,
                [ #(#listeners),* ],
                [ #(#attr),* ],
                [ #(#childs),* ],
                #key,
            )
        });
    }
}

/// =======================================
/// Parse a VElement's Attributes
/// =======================================
struct ElementAttr<const AS: HTML_OR_RSX> {
    element_name: Ident,
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
fn parse_rsx_element_field(
    stream: ParseStream,
    attrs: &mut Vec<ElementAttr<AS_RSX>>,
    listeners: &mut Vec<ElementAttr<AS_RSX>>,
    key: &mut Option<LitStr>,
    el_ref: &mut Option<Expr>,
    element_name: Ident,
) -> Result<()> {
    let name = Ident::parse_any(stream)?;
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
            element_name: element_name.clone(),
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
                    element_name: element_name.clone(),
                });
            }

            return Ok(());
        }
        "key" => {
            *key = Some(stream.parse::<LitStr>()?);
            return Ok(());
        }
        "classes" => {
            todo!("custom class lsit not supported")
        }
        "namespace" => {
            todo!("custom namespace not supported")
        }
        "node_ref" => {
            *el_ref = Some(stream.parse::<Expr>()?);
            return Ok(());
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
        element_name,
    });
    Ok(())
}

impl<const AS: HTML_OR_RSX> ToTokens for ElementAttr<AS> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let el_name = &self.element_name;
        let name_str = self.name.to_string();
        let nameident = &self.name;

        let namespace = match &self.namespace {
            Some(t) => quote! { Some(#t) },
            None => quote! { None },
        };

        match &self.value {
            AttrType::BumpText(value) => tokens.append_all(quote! {
                dioxus_elements::#el_name.#nameident(__cx, format_args_f!(#value))
            }),
            AttrType::FieldTokens(exp) => tokens.append_all(quote! {
                dioxus_elements::#el_name.#nameident(__cx, #exp)
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

// __cx.attr(#name, format_args_f!(#value), #namespace, false)
//
// AttrType::BumpText(value) => tokens.append_all(quote! {
//     __cx.attr(#name, format_args_f!(#value), #namespace, false)
// }),
// __cx.attr(#name_str, #exp, #namespace, false)

// AttrType::FieldTokens(exp) => tokens.append_all(quote! {
//     dioxus_elements::#el_name.#nameident(__cx, format_args_f!(#value))
//     __cx.attr(#name_str, #exp, #namespace, false)
// }),
