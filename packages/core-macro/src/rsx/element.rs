use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    token, Expr, ExprClosure, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
pub struct Element {
    name: Ident,
    key: Option<LitStr>,
    attributes: Vec<ElementAttrNamed>,
    listeners: Vec<ElementAttrNamed>,
    children: Vec<BodyNode>,
    _is_static: bool,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let el_name = Ident::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttrNamed> = vec![];
        let mut listeners: Vec<ElementAttrNamed> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;
        let mut _el_ref = None;

        // todo: more descriptive error handling
        while !content.is_empty() {
            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<Ident>()?;
                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                if name_str.starts_with("on") {
                    if content.peek(token::Brace) {
                        let mycontent;
                        syn::braced!(mycontent in content);

                        listeners.push(ElementAttrNamed {
                            el_name: el_name.clone(),
                            attr: ElementAttr::EventTokens {
                                name,
                                tokens: mycontent.parse()?,
                            },
                        });
                    } else {
                        listeners.push(ElementAttrNamed {
                            el_name: el_name.clone(),
                            attr: ElementAttr::EventClosure {
                                name,
                                closure: content.parse()?,
                            },
                        });
                    };
                } else {
                    match name_str.as_str() {
                        "key" => {
                            key = Some(content.parse()?);
                        }
                        "classes" => {
                            todo!("custom class list not supported")
                        }
                        "namespace" => {
                            todo!("custom namespace not supported")
                        }
                        "node_ref" => {
                            _el_ref = Some(content.parse::<Expr>()?);
                        }
                        _ => {
                            if content.peek(LitStr) {
                                attributes.push(ElementAttrNamed {
                                    el_name: el_name.clone(),
                                    attr: ElementAttr::AttrText {
                                        name,
                                        value: content.parse()?,
                                    },
                                });
                            } else {
                                attributes.push(ElementAttrNamed {
                                    el_name: el_name.clone(),
                                    attr: ElementAttr::AttrExpression {
                                        name,
                                        value: content.parse()?,
                                    },
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
                    attributes.push(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr::CustomAttrText { name, value },
                    });
                } else {
                    let value = content.parse::<Expr>()?;
                    attributes.push(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr::CustomAttrExpression { name, value },
                    });
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
            name: el_name,
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
        let attr = &self.attributes;

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

struct ElementAttrNamed {
    el_name: Ident,
    attr: ElementAttr,
}

impl ToTokens for ElementAttrNamed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr } = self;

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
