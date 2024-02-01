use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_quote, spanned::Spanned, Expr, ExprIf, Ident, LitStr};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeType {
    Named(ElementAttrNamed),
    Spread(Expr),
}

impl AttributeType {
    pub fn start(&self) -> Span {
        match self {
            AttributeType::Named(n) => n.attr.start(),
            AttributeType::Spread(e) => e.span(),
        }
    }

    pub fn matches_attr_name(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named(a), Self::Named(b)) => a.attr.name == b.attr.name,
            _ => false,
        }
    }

    pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Named(a), Self::Named(b)) => a.try_combine(b).map(Self::Named),
            _ => None,
        }
    }

    pub(crate) fn merge_quote(vec: &[&Self]) -> TokenStream2 {
        // split into spread and single attributes
        let mut spread = vec![];
        let mut single = vec![];
        for attr in vec.iter() {
            match attr {
                AttributeType::Named(named) => single.push(named),
                AttributeType::Spread(expr) => spread.push(expr),
            }
        }

        // If all of them are single attributes, create a static slice
        if spread.is_empty() {
            quote! {
                Box::new([
                    #(#single),*
                ])
            }
        } else {
            // Otherwise start with the single attributes and append the spread attributes
            quote! {
                {
                    let mut __attributes = vec![
                        #(#single),*
                    ];
                    #(
                        let mut __spread = #spread;
                        __attributes.append(&mut __spread);
                    )*
                    __attributes.into_boxed_slice()
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ElementAttrNamed {
    pub el_name: ElementName,
    pub attr: ElementAttr,
}

impl Hash for ElementAttrNamed {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.attr.name.hash(state);
    }
}

impl PartialEq for ElementAttrNamed {
    fn eq(&self, other: &Self) -> bool {
        self.attr == other.attr
    }
}

impl Eq for ElementAttrNamed {}

impl ElementAttrNamed {
    pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
        if self.el_name == other.el_name && self.attr.name == other.attr.name {
            if let Some(separator) = self.attr.name.multi_attribute_separator() {
                return Some(ElementAttrNamed {
                    el_name: self.el_name.clone(),
                    attr: ElementAttr {
                        name: self.attr.name.clone(),
                        value: self.attr.value.combine(separator, &other.attr.value),
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

        let ns = |name: &ElementAttrName| match (el_name, name) {
            (ElementName::Ident(i), ElementAttrName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.1 }
            }
            _ => quote! { None },
        };
        let volitile = |name: &ElementAttrName| match (el_name, name) {
            (ElementName::Ident(i), ElementAttrName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.2 }
            }
            _ => quote! { false },
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
            let value = &self.attr.value;
            let is_shorthand_event = match &attr.value {
                ElementAttrValue::Shorthand(s) => s.to_string().starts_with("on"),
                _ => false,
            };

            match &attr.value {
                ElementAttrValue::AttrLiteral(_)
                | ElementAttrValue::AttrExpr(_)
                | ElementAttrValue::Shorthand(_)
                | ElementAttrValue::AttrOptionalExpr { .. }
                    if !is_shorthand_event =>
                {
                    let name = &self.attr.name;
                    let ns = ns(name);
                    let volitile = volitile(name);
                    let attribute = attribute(name);
                    let value = quote! { #value };

                    quote! {
                        dioxus_core::Attribute::new(
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
                            dioxus_elements::events::#name(#tokens)
                        }
                    }
                    ElementAttrName::Custom(_) => unreachable!("Handled elsewhere in the macro"),
                },
                _ => {
                    quote! { dioxus_elements::events::#value(#value) }
                }
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
    /// attribute,
    Shorthand(Ident),
    /// attribute: "value"
    AttrLiteral(IfmtInput),
    /// attribute: if bool { "value" }
    AttrOptionalExpr {
        condition: Expr,
        value: Box<ElementAttrValue>,
    },
    /// attribute: true
    AttrExpr(Expr),
    /// onclick: move |_| {}
    EventTokens(Expr),
}

impl Parse for ElementAttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let element_attr_value = if input.peek(Token![if]) {
            let if_expr = input.parse::<ExprIf>()?;
            if is_if_chain_terminated(&if_expr) {
                ElementAttrValue::AttrExpr(Expr::If(if_expr))
            } else {
                ElementAttrValue::AttrOptionalExpr {
                    condition: *if_expr.cond,
                    value: {
                        let stmts = if_expr.then_branch.stmts;
                        Box::new(syn::parse2(quote! {
                            #(#stmts)*
                        })?)
                    },
                }
            }
        } else if input.peek(LitStr) {
            let value = input.parse()?;
            ElementAttrValue::AttrLiteral(value)
        } else {
            let value = input.parse::<Expr>()?;
            ElementAttrValue::AttrExpr(value)
        };

        Ok(element_attr_value)
    }
}

impl ToTokens for ElementAttrValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementAttrValue::Shorthand(i) => tokens.append_all(quote! { #i }),
            ElementAttrValue::AttrLiteral(lit) => tokens.append_all(quote! { #lit.to_string() }),
            ElementAttrValue::AttrOptionalExpr { condition, value } => {
                tokens.append_all(quote! { if #condition { Some(#value) } else { None } })
            }
            ElementAttrValue::AttrExpr(expr) => tokens.append_all(quote! { #expr }),
            ElementAttrValue::EventTokens(expr) => tokens.append_all(quote! { #expr }),
        }
    }
}

impl ElementAttrValue {
    fn to_str_expr(&self) -> Option<TokenStream2> {
        match self {
            ElementAttrValue::AttrLiteral(lit) => Some(quote!(#lit.to_string())),
            ElementAttrValue::AttrOptionalExpr { value, .. } => value.to_str_expr(),
            ElementAttrValue::AttrExpr(expr) => Some(quote!(#expr.to_string())),
            _ => None,
        }
    }

    fn combine(&self, separator: &str, other: &Self) -> Self {
        match (self, other) {
            (Self::AttrLiteral(lit1), Self::AttrLiteral(lit2)) => {
                let fmt = lit1.clone().join(lit2.clone(), separator);
                Self::AttrLiteral(fmt)
            }
            (Self::AttrLiteral(expr1), Self::AttrExpr(expr2)) => {
                let mut ifmt = expr1.clone();
                ifmt.push_str(separator);
                ifmt.push_expr(expr2.clone());
                Self::AttrLiteral(ifmt)
            }
            (Self::AttrExpr(expr1), Self::AttrLiteral(expr2)) => {
                let mut ifmt = expr2.clone();
                ifmt.push_str(separator);
                ifmt.push_expr(expr1.clone());
                Self::AttrLiteral(ifmt)
            }
            (Self::AttrExpr(expr1), Self::AttrExpr(expr2)) => {
                let mut ifmt = IfmtInput::default();
                ifmt.push_expr(expr1.clone());
                ifmt.push_str(separator);
                ifmt.push_expr(expr2.clone());
                Self::AttrLiteral(ifmt)
            }
            (
                Self::AttrOptionalExpr {
                    condition: condition1,
                    value: value1,
                },
                Self::AttrOptionalExpr {
                    condition: condition2,
                    value: value2,
                },
            ) => {
                let first_as_string = value1.to_str_expr();
                let second_as_string = value2.to_str_expr();
                Self::AttrExpr(parse_quote! {
                    {
                        let mut __combined = String::new();
                        if #condition1 {
                            __combined.push_str(&#first_as_string);
                        }
                        if #condition2 {
                            if __combined.len() > 0 {
                                __combined.push_str(&#separator);
                            }
                            __combined.push_str(&#second_as_string);
                        }
                        __combined
                    }
                })
            }
            (Self::AttrOptionalExpr { condition, value }, other) => {
                let first_as_string = value.to_str_expr();
                let second_as_string = other.to_str_expr();
                Self::AttrExpr(parse_quote! {
                    {
                        let mut __combined = #second_as_string;
                        if #condition {
                            __combined.push_str(&#separator);
                            __combined.push_str(&#first_as_string);
                        }
                        __combined
                    }
                })
            }
            (other, Self::AttrOptionalExpr { condition, value }) => {
                let first_as_string = other.to_str_expr();
                let second_as_string = value.to_str_expr();
                Self::AttrExpr(parse_quote! {
                    {
                        let mut __combined = #first_as_string;
                        if #condition {
                            __combined.push_str(&#separator);
                            __combined.push_str(&#second_as_string);
                        }
                        __combined
                    }
                })
            }
            _ => unreachable!("Invalid combination of attributes"),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementAttrName {
    BuiltIn(Ident),
    Custom(LitStr),
}

impl ElementAttrName {
    fn multi_attribute_separator(&self) -> Option<&'static str> {
        match self {
            ElementAttrName::BuiltIn(i) => match i.to_string().as_str() {
                "class" => Some(" "),
                "style" => Some(";"),
                _ => None,
            },
            ElementAttrName::Custom(_) => None,
        }
    }

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
