use std::fmt::{Display, Formatter};

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{parse_quote, spanned::Spanned, Expr, ExprClosure, ExprIf, Ident, LitStr};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeType {
    /// An attribute that is known
    Named(ElementAttrNamed),

    /// An attribute that's being spread in via the `..` syntax
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

    pub fn as_static_str_literal(&self) -> Option<(&ElementAttrName, &IfmtInput)> {
        match self {
            AttributeType::Named(ElementAttrNamed {
                attr:
                    ElementAttr {
                        value: ElementAttrValue::AttrLiteral(value),
                        name,
                    },
                ..
            }) if value.is_static() => Some((name, value)),
            _ => None,
        }
    }

    pub fn is_static_str_literal(&self) -> bool {
        self.as_static_str_literal().is_some()
    }
}

#[derive(Clone, Debug)]
pub struct ElementAttrNamed {
    pub el_name: ElementName,
    pub attr: ElementAttr,
    // If this is the last attribute of an element and it doesn't have a tailing comma,
    // we add hints so that rust analyzer completes it either as an attribute or element
    pub(crate) followed_by_comma: bool,
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
    /// Create a new ElementAttrNamed
    pub fn new(el_name: ElementName, attr: ElementAttr) -> Self {
        Self {
            el_name,
            attr,
            followed_by_comma: true,
        }
    }

    pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
        if self.el_name == other.el_name && self.attr.name == other.attr.name {
            if let Some(separator) = self.attr.name.multi_attribute_separator() {
                return Some(ElementAttrNamed {
                    el_name: self.el_name.clone(),
                    attr: ElementAttr {
                        name: self.attr.name.clone(),
                        value: self.attr.value.combine(separator, &other.attr.value),
                    },
                    followed_by_comma: self.followed_by_comma || other.followed_by_comma,
                });
            }
        }
        None
    }

    /// If this is the last attribute of an element and it doesn't have a tailing comma,
    /// we add hints so that rust analyzer completes it either as an attribute or element
    fn completion_hints(&self) -> TokenStream2 {
        let ElementAttrNamed {
            el_name,
            attr,
            followed_by_comma,
        } = self;

        // If there is a trailing comma, rust analyzer does a good job of completing the attribute by itself
        if *followed_by_comma {
            return quote! {};
        }
        // Only add hints if the attribute is:
        // - a built in attribute (not a literal)
        // - an build in element (not a custom element)
        // - a shorthand attribute
        let (
            ElementName::Ident(el),
            ElementAttrName::BuiltIn(name),
            ElementAttrValue::Shorthand(_),
        ) = (el_name, &attr.name, &attr.value)
        else {
            return quote! {};
        };
        // If the attribute is a shorthand attribute, but it is an event handler, rust analyzer already does a good job of completing the attribute by itself
        if name.to_string().starts_with("on") {
            return quote! {};
        }

        quote! {
            {
                #[allow(dead_code)]
                #[doc(hidden)]
                mod __completions {
                    // Autocomplete as an attribute
                    pub use super::dioxus_elements::#el::*;
                    // Autocomplete as an element
                    pub use super::dioxus_elements::elements::completions::CompleteWithBraces::*;
                    fn ignore() {
                        #name
                    }
                }
            }
        }
    }
}

impl ToTokens for ElementAttrNamed {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ElementAttrNamed { el_name, attr, .. } = self;

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
                        let event_tokens_is_closure =
                            syn::parse2::<ExprClosure>(tokens.to_token_stream()).is_ok();
                        let function_name =
                            quote_spanned! { tokens.span() => dioxus_elements::events::#name };
                        let function = if event_tokens_is_closure {
                            // If we see an explicit closure, we can call the `call_with_explicit_closure` version of the event for better type inference
                            quote_spanned! { tokens.span() => #function_name::call_with_explicit_closure }
                        } else {
                            function_name
                        };
                        quote_spanned! { tokens.span() =>
                            #function(#tokens)
                        }
                    }
                    ElementAttrName::Custom(_) => unreachable!("Handled elsewhere in the macro"),
                },
                _ => {
                    quote_spanned! { value.span() => dioxus_elements::events::#value(#value) }
                }
            }
        };

        let completion_hints = self.completion_hints();
        tokens.append_all(quote! {
            {
                #completion_hints
                #attribute
            }
        });
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ElementAttr {
    pub name: ElementAttrName,
    pub value: ElementAttrValue,
}

impl ElementAttr {
    pub fn can_be_shorthand(&self) -> bool {
        // If it's a shorthand...
        if matches!(self.value, ElementAttrValue::Shorthand(_)) {
            return true;
        }

        // If it's in the form of attr: attr, return true
        if let ElementAttrValue::AttrExpr(Expr::Path(path)) = &self.value {
            if let ElementAttrName::BuiltIn(name) = &self.name {
                if path.path.segments.len() == 1 && &path.path.segments[0].ident == name {
                    return true;
                }
            }
        }

        false
    }
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
    /// Create a new ElementAttrValue::Shorthand from an Ident and normalize the identifier
    pub(crate) fn shorthand(name: &Ident) -> Self {
        Self::Shorthand(normalize_raw_ident(name))
    }

    pub fn is_shorthand(&self) -> bool {
        matches!(self, ElementAttrValue::Shorthand(_))
    }

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

// Create and normalize a built-in attribute name
// If the identifier is a reserved keyword, this method will create a raw identifier
fn normalize_raw_ident(ident: &Ident) -> Ident {
    if syn::parse2::<syn::Ident>(ident.to_token_stream()).is_err() {
        syn::Ident::new_raw(&ident.to_string(), ident.span())
    } else {
        ident.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementAttrName {
    BuiltIn(Ident),
    Custom(LitStr),
}

impl ElementAttrName {
    pub(crate) fn built_in(name: &Ident) -> Self {
        Self::BuiltIn(normalize_raw_ident(name))
    }

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
