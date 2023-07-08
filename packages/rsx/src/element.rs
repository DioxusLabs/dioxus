use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Expr, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Element {
    pub name: ElementName,
    pub key: Option<IfmtInput>,
    pub attributes: Vec<ElementAttrNamed>,
    pub children: Vec<BodyNode>,
    pub brace: syn::token::Brace,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let el_name = ElementName::parse(stream)?;

        // parse the guts
        let content: ParseBuffer;
        let brace = syn::braced!(content in stream);

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

                if content.peek(LitStr) {
                    let value = content.parse()?;
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

                let name_str = name.to_string();
                content.parse::<Token![:]>()?;

                // The span of the content to be parsed,
                // for example the `hi` part of `class: "hi"`.
                let span = content.span();

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
                    missing_trailing_comma!(span);
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
            brace,
        })
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let children = &self.children;

        let key = match &self.key {
            Some(ty) => quote! { Some(#ty) },
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
                #name,
                __cx.bump().alloc([ #(#listeners),* ]),
                __cx.bump().alloc([ #(#attr),* ]),
                __cx.bump().alloc([ #(#children),* ]),
                #key,
            )
        });
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementName {
    Ident(Ident),
    Custom(LitStr),
}

impl ElementName {
    pub(crate) fn tag_name(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::TAG_NAME },
            ElementName::Custom(s) => quote! { #s },
        }
    }
}

impl ElementName {
    pub fn span(&self) -> Span {
        match self {
            ElementName::Ident(i) => i.span(),
            ElementName::Custom(s) => s.span(),
        }
    }
}

impl PartialEq<&str> for ElementName {
    fn eq(&self, other: &&str) -> bool {
        match self {
            ElementName::Ident(i) => i == *other,
            ElementName::Custom(s) => s.value() == *other,
        }
    }
}

impl Display for ElementName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementName::Ident(i) => write!(f, "{}", i),
            ElementName::Custom(s) => write!(f, "{}", s.value()),
        }
    }
}

impl Parse for ElementName {
    fn parse(stream: ParseStream) -> Result<Self> {
        let raw = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(stream)?;
        if raw.len() == 1 {
            Ok(ElementName::Ident(raw.into_iter().next().unwrap()))
        } else {
            let span = raw.span();
            let tag = raw
                .into_iter()
                .map(|ident| ident.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let tag = LitStr::new(&tag, span);
            Ok(ElementName::Custom(tag))
        }
    }
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementName::Ident(i) => tokens.append_all(quote! { dioxus_elements::#i }),
            ElementName::Custom(s) => tokens.append_all(quote! { #s }),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementAttr {
    /// `attribute: "value"`
    AttrText { name: Ident, value: IfmtInput },

    /// `attribute: true`
    AttrExpression { name: Ident, value: Expr },

    /// `"attribute": "value"`
    CustomAttrText { name: LitStr, value: IfmtInput },

    /// `"attribute": true`
    CustomAttrExpression { name: LitStr, value: Expr },

    // /// onclick: move |_| {}
    // EventClosure { name: Ident, closure: ExprClosure },
    /// onclick: {}
    EventTokens { name: Ident, tokens: Expr },
}

impl ElementAttr {
    pub fn start(&self) -> Span {
        match self {
            ElementAttr::AttrText { name, .. } => name.span(),
            ElementAttr::AttrExpression { name, .. } => name.span(),
            ElementAttr::CustomAttrText { name, .. } => name.span(),
            ElementAttr::CustomAttrExpression { name, .. } => name.span(),
            ElementAttr::EventTokens { name, .. } => name.span(),
        }
    }

    pub fn is_expr(&self) -> bool {
        matches!(
            self,
            ElementAttr::AttrExpression { .. }
                | ElementAttr::CustomAttrExpression { .. }
                | ElementAttr::EventTokens { .. }
        )
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ElementAttrNamed {
    pub el_name: ElementName,
    pub attr: ElementAttr,
}

impl ToTokens for ElementAttrNamed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr } = self;

        let ns = |name| match el_name {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::#name.1 },
            ElementName::Custom(_) => quote! { None },
        };
        let volitile = |name| match el_name {
            ElementName::Ident(_) => quote! { #el_name::#name.2 },
            ElementName::Custom(_) => quote! { false },
        };
        let attribute = |name: &Ident| match el_name {
            ElementName::Ident(_) => quote! { #el_name::#name.0 },
            ElementName::Custom(_) => {
                let as_string = name.to_string();
                quote!(#as_string)
            }
        };

        let attribute = match attr {
            ElementAttr::AttrText { name, value } => {
                let ns = ns(name);
                let volitile = volitile(name);
                let attribute = attribute(name);
                quote! {
                    __cx.attr(
                        #attribute,
                        #value,
                        #ns,
                        #volitile
                    )
                }
            }
            ElementAttr::AttrExpression { name, value } => {
                let ns = ns(name);
                let volitile = volitile(name);
                let attribute = attribute(name);
                quote! {
                    __cx.attr(
                        #attribute,
                        #value,
                        #ns,
                        #volitile
                    )
                }
            }
            ElementAttr::CustomAttrText { name, value } => {
                quote! {
                    __cx.attr(
                        #name,
                        #value,
                        None,
                        false
                    )
                }
            }
            ElementAttr::CustomAttrExpression { name, value } => {
                quote! {
                    __cx.attr(
                        #name,
                        #value,
                        None,
                        false
                    )
                }
            }
            ElementAttr::EventTokens { name, tokens } => {
                quote! {
                    dioxus_elements::events::#name(__cx, #tokens)
                }
            }
        };

        tokens.append_all(attribute);
    }
}

// ::dioxus::core::Attribute {
//     name: stringify!(#name),
//     namespace: None,
//     volatile: false,
//     mounted_node: Default::default(),
//     value: ::dioxus::core::AttributeValue::Text(#value),
// }
