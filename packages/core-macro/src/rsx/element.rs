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
    key: Option<LitStr>,
    attributes: Vec<ElementAttr>,
    listeners: Vec<ElementAttr>,
    children: Vec<BodyNode>,
    _is_static: bool,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = Ident::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttr> = vec![];
        let mut listeners: Vec<ElementAttr> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;
        let mut el_ref = None;

        while !content.is_empty() {
            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = Ident::parse_any(stream)?;
                let name_str = name.to_string();
                stream.parse::<Token![:]>()?;

                if name_str.starts_with("on") {
                    if stream.peek(token::Brace) {
                        let content;
                        syn::braced!(content in stream);

                        listeners.push(ElementAttr::EventTokens {
                            name,
                            tokens: content.parse()?,
                        });
                    } else {
                        listeners.push(ElementAttr::EventClosure {
                            name,
                            closure: content.parse()?,
                        });
                    };
                } else {
                    match name_str.as_str() {
                        "key" => {
                            key = Some(stream.parse()?);
                        }
                        "classes" => {
                            todo!("custom class list not supported")
                        }
                        "namespace" => {
                            todo!("custom namespace not supported")
                        }
                        "node_ref" => {
                            el_ref = Some(stream.parse::<Expr>()?);
                        }
                        _ => {
                            if stream.peek(LitStr) {
                                listeners.push(ElementAttr::AttrText {
                                    name,
                                    value: content.parse()?,
                                });
                            } else {
                                listeners.push(ElementAttr::AttrExpression {
                                    name,
                                    value: content.parse()?,
                                });
                            }
                        }
                    }
                }
            } else if content.peek(LitStr) && content.peek2(Token![:]) {
                let name = content.parse::<LitStr>()?;
                content.parse::<Token![:]>()?;

                if content.peek(LitStr) {
                    let value = content.parse::<LitStr>()?;
                    attributes.push(ElementAttr::CustomAttrText { name, value });
                } else {
                    let value = content.parse::<Expr>()?;
                    attributes.push(ElementAttr::CustomAttrExpression { name, value });
                }
            } else {
                children.push(content.parse::<BodyNode>()?);
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
            _is_static: false,
        })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let childs = &self.children;
        let listeners = &self.listeners;

        let attr = self.attributes.iter().map(|x| ElementAttrNamed {
            attr: x,
            el_name: name,
        });

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

enum ElementAttr {
    // attribute: "valuee {}"
    AttrText { name: Ident, value: LitStr },

    // attribute: true,
    AttrExpression { name: Ident, value: Expr },

    // "attribute": "value {}"
    CustomAttrText { name: LitStr, value: LitStr },

    // "attribute": true,
    CustomAttrExpression { name: LitStr, value: Expr },

    // onclick: move |_| {}
    EventClosure { name: Ident, closure: ExprClosure },

    // onclick: {}
    EventTokens { name: Ident, tokens: Expr },
}

impl ToTokens for ElementAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // weird requirment
        todo!()
    }
}

struct ElementAttrNamed<'a> {
    el_name: &'a Ident,
    attr: &'a ElementAttr,
}

impl ToTokens for ElementAttrNamed<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr } = *self;

        let toks = match attr {
            ElementAttr::AttrText { name, value } => {
                quote! {
                    dioxus_elements::#el_name.#name(__cx, format_args_f!(#value))
                }
            }
            ElementAttr::AttrExpression { name, value } => {
                quote! {
                    dioxus_elements::#el_name.#name(__cx, #value)
                }
            }

            ElementAttr::CustomAttrText { name, value } => {
                quote! { __cx.attr( #name, format_args_f!(#value), None, false ) }
            }
            ElementAttr::CustomAttrExpression { name, value } => {
                quote! { __cx.attr( #name, format_args_f!(#value), None, false ) }
            }

            ElementAttr::EventClosure { name, closure } => {
                quote! {
                    dioxus::events::on::#name(__cx, #closure)
                }
            }
            ElementAttr::EventTokens { name, tokens } => {
                quote! {
                    dioxus::events::on::#name(__cx, #tokens)
                }
            }
        };

        tokens.append_all(toks);
    }
}
