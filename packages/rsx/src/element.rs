use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    Error, Expr, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
#[derive(PartialEq, Eq)]
pub struct Element {
    pub name: Ident,
    pub key: Option<LitStr>,
    pub attributes: Vec<ElementAttrNamed>,
    pub children: Vec<BodyNode>,
    pub _is_static: bool,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let el_name = Ident::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attributes: Vec<ElementAttrNamed> = vec![];
        let mut children: Vec<BodyNode> = vec![];
        let mut key = None;
        let mut _el_ref = None;

        // parse fields with commas
        // break when we don't get this pattern anymore
        // start parsing bodynodes
        // "def": 456,
        // abc: 123,
        loop {
            // Parse the raw literal fields
            if content.peek(LitStr) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<LitStr>()?;
                let ident = name.clone();

                content.parse::<Token![:]>()?;

                if content.peek(LitStr) && content.peek2(Token![,]) {
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

                if content.is_empty() {
                    break;
                }

                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(ident.span());
                }
                continue;
            }

            if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                let name = content.parse::<Ident>()?;
                let ident = name.clone();

                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                if name_str.starts_with("on") {
                    attributes.push(ElementAttrNamed {
                        el_name: el_name.clone(),
                        attr: ElementAttr::EventTokens {
                            name,
                            tokens: content.parse()?,
                        },
                    });
                } else {
                    match name_str.as_str() {
                        "key" => {
                            key = Some(content.parse()?);
                        }
                        "classes" => todo!("custom class list not supported yet"),
                        // "namespace" => todo!("custom namespace not supported yet"),
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

                if content.is_empty() {
                    break;
                }

                // todo: add a message saying you need to include commas between fields
                if content.parse::<Token![,]>().is_err() {
                    missing_trailing_comma!(ident.span());
                }
                continue;
            }

            break;
        }

        while !content.is_empty() {
            if (content.peek(LitStr) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            if (content.peek(Ident) && content.peek2(Token![:])) && !content.peek3(Token![:]) {
                attr_after_element!(content.span());
            }

            children.push(content.parse::<BodyNode>()?);
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
            _is_static: false,
        })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let children = &self.children;

        let key = match &self.key {
            Some(ty) => quote! { Some(format_args_f!(#ty)) },
            None => quote! { None },
        };

        let listeners = self
            .attributes
            .iter()
            .filter(|f| matches!(f.attr, ElementAttr::EventTokens { .. }));

        let attr = self
            .attributes
            .iter()
            .filter(|f| !matches!(f.attr, ElementAttr::EventTokens { .. }));

        tokens.append_all(quote! {
            __cx.element(
                dioxus_elements::#name,
                __cx.bump().alloc([ #(#listeners),* ]),
                __cx.bump().alloc([ #(#attr),* ]),
                __cx.bump().alloc([ #(#children),* ]),
                #key,
            )
        });
    }
}

#[derive(PartialEq, Eq)]
pub enum ElementAttr {
    /// attribute: "valuee {}"
    AttrText { name: Ident, value: LitStr },

    /// attribute: true,
    AttrExpression { name: Ident, value: Expr },

    /// "attribute": "value {}"
    CustomAttrText { name: LitStr, value: LitStr },

    /// "attribute": true,
    CustomAttrExpression { name: LitStr, value: Expr },

    // /// onclick: move |_| {}
    // EventClosure { name: Ident, closure: ExprClosure },
    /// onclick: {}
    EventTokens { name: Ident, tokens: Expr },
}
impl ElementAttr {
    pub fn is_expr(&self) -> bool {
        matches!(
            self,
            ElementAttr::AttrExpression { .. }
                | ElementAttr::CustomAttrExpression { .. }
                | ElementAttr::EventTokens { .. }
        )
    }
}

#[derive(PartialEq, Eq)]
pub struct ElementAttrNamed {
    pub el_name: Ident,
    pub attr: ElementAttr,
}

impl ToTokens for ElementAttrNamed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr } = self;

        tokens.append_all(match attr {
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
                quote! {
                    __cx.attr( #name, format_args_f!(#value), None, false )
                }
            }
            ElementAttr::CustomAttrExpression { name, value } => {
                quote! {
                    __cx.attr( #name, format_args_f!(#value), None, false )
                }
            }
            // ElementAttr::EventClosure { name, closure } => {
            //     quote! {
            //         dioxus_elements::on::#name(__cx, #closure)
            //     }
            // }
            ElementAttr::EventTokens { name, tokens } => {
                quote! {
                    dioxus_elements::on::#name(__cx, #tokens)
                }
            }
        });
    }
}
