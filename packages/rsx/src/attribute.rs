use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, Ident, LitStr, Result, Token,
};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ElementAttrNamed {
    pub el_name: ElementName,
    pub attr: ElementAttr,
}

impl ElementAttrNamed {
    pub(crate) fn try_combine(&self, other: Self) -> Option<Self> {
        if self.el_name == other.el_name && self.attr.name == other.attr.name {
            if let Some(separator) = todo!() {
                return Some(ElementAttrNamed {
                    el_name: self.el_name.clone(),
                    attr: ElementAttr {
                        name: self.attr.name.clone(),
                        value: self.attr.value.combine(separator, other.attr.value),
                    },
                });
            }
        }
        None
    }
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
        let attribute = |name: &ElementAttrName| match name {
            ElementAttrName::BuiltIn(name) => match el_name {
                ElementName::Ident(_) => quote! { #el_name::#name.0 },
                ElementName::Custom(_) => {
                    let as_string = name.to_string();
                    quote!(#as_string)
                }
            },
            ElementAttrName::Custom(s) => quote! { #s },
        };

        let attribute = {
            match &attr.value {
                ElementAttrValue::AttrLiteral(_) | ElementAttrValue::AttrExpr(_) => {
                    let name = &self.attr.name;
                    let ns = ns(name);
                    let volitile = volitile(name);
                    let attribute = attribute(name);
                    let value = match &self.attr.value {
                        ElementAttrValue::AttrLiteral(lit) => quote! { #lit },
                        ElementAttrValue::AttrExpr(expr) => quote! { #expr },
                        _ => unreachable!(),
                    };
                    quote! {
                        __cx.attr(
                            #attribute,
                            #value,
                            #ns,
                            #volitile
                        )
                    }
                }
                ElementAttrValue::EventTokens(tokens) => match &self.attr.name {
                    ElementAttrName::BuiltIn(name) => {
                        quote! {
                            dioxus_elements::events::#name(__cx, #tokens)
                        }
                    }
                    ElementAttrName::Custom(_) => todo!(),
                },
            }
        };

        tokens.append_all(attribute);
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ElementAttr {
    pub name: ElementAttrName,
    pub value: ElementAttrValue,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementAttrValue {
    /// attribute: "value"
    AttrLiteral(IfmtInput),
    /// attribute: true
    AttrExpr(Expr),
    /// onclick: move |_| {}
    EventTokens(Expr),
}

impl ElementAttrValue {
    fn combine(&self, separator: &str, other: Self) -> Self {
        todo!()
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementAttrName {
    BuiltIn(Ident),
    Custom(LitStr),
}

impl ElementAttrName {
    pub fn start(&self) -> Span {
        match self {
            ElementAttrName::BuiltIn(i) => i.span(),
            ElementAttrName::Custom(s) => s.span(),
        }
    }
}

impl ToTokens for ElementAttrName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementAttrName::BuiltIn(i) => tokens.append_all(quote! { #i }),
            ElementAttrName::Custom(s) => tokens.append_all(quote! { #s }),
        }
    }
}

impl Display for ElementAttrName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementAttrName::BuiltIn(i) => write!(f, "{}", i),
            ElementAttrName::Custom(s) => write!(f, "{}", s.value()),
        }
    }
}

impl ElementAttr {
    pub fn start(&self) -> Span {
        self.name.start()
    }

    pub fn is_expr(&self) -> bool {
        matches!(
            self,
            ElementAttr {
                value: ElementAttrValue::AttrExpr(_) | ElementAttrValue::EventTokens(_),
                ..
            }
        )
    }
}
