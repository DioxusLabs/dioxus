use std::fmt::Display;

use crate::{innerlude::*, partial_closure::PartialClosure, HotReloadingContext};
use dioxus_core::prelude::TemplateAttribute;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer, ParseStream},
    spanned::Spanned,
    token::{self, Brace},
    AngleBracketedGenericArguments, Expr, ExprClosure, ExprIf, Ident, Lit, LitStr, PatLit,
    PathArguments, Token,
};

use super::literal::HotLiteral;

/// A property value in the from of a `name: value` pair with an optional comma.
/// Note that the colon and value are optional in the case of shorthand attributes. We keep them around
/// to support "lossless" parsing in case that ever might be useful.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Attribute {
    pub name: AttributeName,
    pub colon: Option<Token![:]>,
    pub value: AttributeValue,
    pub comma: Option<Token![,]>,
    pub dyn_idx: DynIdx,
}

impl Parse for Attribute {
    fn parse(content: ParseStream) -> syn::Result<Self> {
        // if there's an ident not followed by a colon, it's a shorthand attribute
        if content.peek(Ident) && !content.peek2(Token![:]) {
            let ident = content.parse::<Ident>()?;
            let comma = if !content.is_empty() {
                Some(content.parse::<Token![,]>()?)
            } else {
                None
            };

            return Ok(Attribute {
                name: AttributeName::BuiltIn(ident.clone()),
                colon: None,
                value: AttributeValue::Shorthand(ident),
                comma,
                dyn_idx: DynIdx::default(),
            });
        }

        // Parse the name as either a known or custom attribute
        let name = match content.peek(LitStr) {
            true => AttributeName::Custom(content.parse::<LitStr>()?),
            false => AttributeName::BuiltIn(content.parse::<Ident>()?),
        };

        // Ensure there's a colon
        let colon = Some(content.parse::<Token![:]>()?);

        // todo: make this cleaner please
        // if statements in attributes get automatic closing in some cases
        let value = if content.peek(Token![if]) {
            let if_expr = content.parse::<ExprIf>()?;
            if is_if_chain_terminated(&if_expr) {
                AttributeValue::AttrExpr(Expr::If(if_expr))
            } else {
                AttributeValue::AttrOptionalExpr {
                    condition: *if_expr.cond,
                    value: {
                        let stmts = &if_expr.then_branch.stmts;

                        if stmts.len() != 1 {
                            return Err(syn::Error::new(
                                if_expr.then_branch.span(),
                                "Expected a single statement in the if block",
                            ));
                        }

                        // either an ifmt or an expr in the block
                        let stmt = &stmts[0];

                        // Either it's a valid ifmt or an expression
                        match stmt {
                            syn::Stmt::Expr(exp, None) => {
                                // Try parsing the statement as an IfmtInput by passing it through tokens
                                let value: Result<HotLiteral, syn::Error> =
                                    syn::parse2(quote! { #exp });

                                match value {
                                    Ok(res) => Box::new(AttributeValue::AttrLiteral(res)),
                                    Err(_) => Box::new(AttributeValue::AttrExpr(exp.clone())),
                                }
                            }
                            _ => {
                                return Err(syn::Error::new(stmt.span(), "Expected an expression"))
                            }
                        }
                    },
                }
            }
        } else if HotLiteral::peek(&content) {
            let value = content.parse()?;
            AttributeValue::AttrLiteral(value)
        } else if content.peek(Token![move]) || content.peek(Token![|]) {
            // todo: add better partial expansion for closures - that's why we're handling them differently here
            let value: PartialClosure = content.parse()?;
            AttributeValue::EventTokens(value)
        } else {
            let value = content.parse::<Expr>()?;
            AttributeValue::AttrExpr(value)
        };

        let comma = if !content.is_empty() {
            Some(content.parse::<Token![,]>()?) // <--- diagnostics...
        } else {
            None
        };

        let attr = Attribute {
            name,
            value,
            colon,
            comma,
            dyn_idx: DynIdx::default(),
        };

        Ok(attr)
    }
}

impl Attribute {
    pub fn span(&self) -> proc_macro2::Span {
        self.name.span()
    }

    /// Get a score of hotreloadability of this attribute with another attribute
    ///
    /// usize::max is a perfect score and an immediate match
    /// 0 is no match
    /// All other scores are relative to the other scores
    pub fn hotreload_score(&self, other: &Attribute) -> usize {
        if self.name != other.name {
            return 0;
        }

        match (&self.value, &other.value) {
            (AttributeValue::AttrLiteral(lit), AttributeValue::AttrLiteral(other_lit)) => {
                match (&lit.value, &lit.value) {
                    (HotLiteralType::Fmted(a), HotLiteralType::Fmted(b)) => {
                        todo!()
                    }
                    (othera, otherb) if othera == otherb => usize::MAX,
                    _ => 0,
                }
            }
            (othera, otherb) if othera == otherb => 1,
            _ => 0,
        }
    }

    pub fn as_lit(&self) -> Option<&HotLiteral> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => Some(lit),
            _ => None,
        }
    }

    /// Run this closure against the attribute if it's hotreloadable
    pub fn with_hr(&self, f: impl FnOnce(&HotLiteral)) {
        if let AttributeValue::AttrLiteral(ifmt) = &self.value {
            f(ifmt);
        }
    }

    pub fn ifmt(&self) -> Option<&IfmtInput> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => match &lit.value {
                HotLiteralType::Fmted(input) => Some(input),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_static_str_literal(&self) -> Option<(&AttributeName, &IfmtInput)> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => match &lit.value {
                HotLiteralType::Fmted(input) if input.is_static() => Some((&self.name, input)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn is_static_str_literal(&self) -> bool {
        self.as_static_str_literal().is_some()
    }

    pub fn to_template_attribute<Ctx: HotReloadingContext>(
        &self,
        rust_name: &str,
    ) -> TemplateAttribute {
        // If it's a dynamic node, just return it
        // For dynamic attributes, we need to check the mapping to see if that mapping exists
        // todo: one day we could generate new dynamic attributes on the fly if they're a literal,
        // or something sufficiently serializable
        //  (ie `checked`` being a bool and bools being interpretable)
        //
        // For now, just give up if that attribute doesn't exist in the mapping
        if !self.is_static_str_literal() {
            let id = self.dyn_idx.get();
            return TemplateAttribute::Dynamic { id };
        }

        // Otherwise it's a static node and we can build it
        let (_name, value) = self.as_static_str_literal().unwrap();
        let attribute_name_rust = self.name.to_string();

        let (name, namespace) = Ctx::map_attribute(&rust_name, &attribute_name_rust)
            .unwrap_or((intern(attribute_name_rust.as_str()), None));

        TemplateAttribute::Static {
            name,
            namespace,
            value: intern(value.to_static().unwrap().as_str()),
        }
    }

    pub fn rendered_as_dynamic_attr(&self, el_name: &ElementName) -> TokenStream2 {
        let mut tokens = TokenStream2::new();

        let ns = |name: &AttributeName| match (el_name, name) {
            (ElementName::Ident(i), AttributeName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.1 }
            }
            _ => quote! { None },
        };

        let volatile = |name: &AttributeName| match (el_name, name) {
            (ElementName::Ident(i), AttributeName::BuiltIn(_)) => {
                quote! { dioxus_elements::#i::#name.2 }
            }
            _ => quote! { false },
        };

        todo!()
        // let attribute = {
        //     let value = &self.value;
        //     let is_shorthand_event = match &self.value {
        //         AttributeValue::Shorthand(s) => s.to_string().starts_with("on"),
        //         _ => false,
        //     };

        //     match &self.value {
        //         AttributeValue::AttrLiteral(_)
        //         | AttributeValue::AttrExpr(_)
        //         | AttributeValue::Shorthand(_)
        //         | AttributeValue::AttrOptionalExpr { .. }
        //             if !is_shorthand_event =>
        //         {
        //             let name = &self.name;
        //             let ns = ns(name);
        //             let volitile = volitile(name);
        //             let attribute = attribute(name);
        //             let value = quote! { #value };

        //             quote! {
        //                 dioxus_core::Attribute::new(
        //                     #attribute,
        //                     #value,
        //                     #ns,
        //                     #volitile
        //                 )
        //             }
        //         }
        //         AttributeValue::EventTokens(tokens) => match &self.name {
        //             AttributeName::BuiltIn(name) => {
        //                 let event_tokens_is_closure =
        //                     syn::parse2::<ExprClosure>(tokens.to_token_stream()).is_ok();
        //                 let function_name =
        //                     quote_spanned! { tokens.span() => dioxus_elements::events::#name };
        //                 let function = if event_tokens_is_closure {
        //                     // If we see an explicit closure, we can call the `call_with_explicit_closure` version of the event for better type inference
        //                     quote_spanned! { tokens.span() => #function_name::call_with_explicit_closure }
        //                 } else {
        //                     function_name
        //                 };
        //                 quote_spanned! { tokens.span() =>
        //                     #function(#tokens)
        //                 }
        //             }
        //             AttributeName::Custom(_) => unreachable!("Handled elsewhere in the macro"),
        //         },
        //         _ => {
        //             quote_spanned! { value.span() => dioxus_elements::events::#value(#value) }
        //         }
        //     }
        // };

        // let completion_hints = self.completion_hints(el_name);
        // quote! {
        //     {
        //         #completion_hints
        //         #attribute
        //     }
        // }
        // .to_token_stream()

        // let attribute = |name: &AttributeName| match name {
        //     AttributeName::BuiltIn(name) => match el_name {
        //         ElementName::Ident(_) => quote! { dioxus_elements::#el_name::#name.0 },
        //         ElementName::Custom(_) => {
        //             let as_string = name.to_string();
        //             quote!(#as_string)
        //         }
        //     },
        //     AttributeName::Custom(s) => quote! { #s },
        // };

        // let value = &self.value;
        // let name = &self.name;

        // let is_event = match &self.name {
        //     AttributeName::BuiltIn(name) => name.to_string().starts_with("on"),
        //     _ => false,
        // };

        // // If all of them are single attributes, create a static slice
        // if spread.is_empty() {
        //     quote! {
        //         Box::new([
        //             #(#single),*
        //         ])
        //     }
        // } else {
        //     // Otherwise start with the single attributes and append the spread attributes
        //     quote! {
        //         {
        //             let mut __attributes = vec![
        //                 #(#single),*
        //             ];
        //             #(
        //                 let mut __spread = #spread;
        //                 __attributes.append(&mut __spread);
        //             )*
        //             __attributes.into_boxed_slice()
        //         }
        //     }
        // }

        // If it's an event, we need to wrap it in the event form and then just return that
        // if is_event {
        //     quote! {
        //         Box::new([
        //             dioxus_elements::events::#name(#value)
        //         ])
        //     }
        // } else {
        //     let ns = ns(name);
        //     let volatile = volatile(name);
        //     let attribute = attribute(name);
        //     let value = quote! { #value };

        //     quote! {
        //         Box::new([
        //             dioxus_core::Attribute::new(
        //                 #attribute,
        //                 #value,
        //                 #ns,
        //                 #volatile
        //             )
        //         ])
        //     }
        // }
    }

    pub fn start(&self) -> proc_macro2::Span {
        self.span()
    }

    pub fn can_be_shorthand(&self) -> bool {
        // If it's a shorthand...
        if matches!(self.value, AttributeValue::Shorthand(_)) {
            return true;
        }

        // If it's in the form of attr: attr, return true
        if let AttributeValue::AttrExpr(Expr::Path(path)) = &self.value {
            if let AttributeName::BuiltIn(name) = &self.name {
                if path.path.segments.len() == 1 && &path.path.segments[0].ident == name {
                    return true;
                }
            }
        }

        false
    }

    // pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
    //     if self.name == other.name {
    //         if let Some(separator) = self.name.multi_attribute_separator() {
    //             return Some(Attribute {
    //                 name: self.name.clone(),
    //                 colon: self.colon.clone(),
    //                 value: self.value.combine(separator, &other.value),
    //                 comma: self.comma.clone().or(other.comma.clone()),
    //                 dyn_idx: self.dyn_idx.clone(),
    //             });
    //         }
    //     }

    //     None
    // }

    /// If this is the last attribute of an element and it doesn't have a tailing comma,
    /// we add hints so that rust analyzer completes it either as an attribute or element
    fn completion_hints(&self, el_name: &ElementName) -> TokenStream2 {
        let Attribute {
            name, value, comma, ..
        } = self;

        // If there is a trailing comma, rust analyzer does a good job of completing the attribute by itself
        if comma.is_some() {
            return quote! {};
        }

        // Only add hints if the attribute is:
        // - a built in attribute (not a literal)
        // - an build in element (not a custom element)
        // - a shorthand attribute
        let (ElementName::Ident(el), AttributeName::BuiltIn(name), AttributeValue::Shorthand(_)) =
            (&el_name, &name, &value)
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

    pub fn matches_attr_name(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeName {
    /// an attribute in the form of `name: value`
    BuiltIn(Ident),

    /// an attribute in the form of `"name": value` - notice that the name is a string literal
    /// this is to allow custom attributes in the case of missing built-in attributes
    ///
    /// we might want to change this one day to be ticked or something and simply a boolean
    Custom(LitStr),
}

impl AttributeName {
    pub fn ident_to_str(&self) -> String {
        match self {
            Self::Custom(lit) => lit.value(),
            Self::BuiltIn(ident) => ident.to_string(),
        }
    }

    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Custom(lit) => lit.span(),
            Self::BuiltIn(ident) => ident.span(),
        }
    }

    /// we have some special casing for the separator of certain attributes
    /// ... I don't really like special casing things in the rsx! macro since it's supposed to be
    /// agnostic to the renderer. To be "correct" we'd need to get the separate from the definition.
    ///
    /// sooo todo: make attribute sepaerator a part of the attribute definition
    fn multi_attribute_separator(&self) -> Option<&'static str> {
        match &self {
            AttributeName::BuiltIn(i) => match i.to_string().as_str() {
                "class" => Some(" "),
                "style" => Some(";"),
                _ => None,
            },
            AttributeName::Custom(_) => None,
        }
    }
}

impl Display for AttributeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(lit) => write!(f, "{}", lit.value()),
            Self::BuiltIn(ident) => write!(f, "{}", ident),
        }
    }
}

impl ToTokens for AttributeName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Custom(lit) => lit.to_tokens(tokens),
            Self::BuiltIn(ident) => ident.to_tokens(tokens),
        }
    }
}

// ..spread attribute
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Spread {
    pub dots: Token![..],
    pub expr: Expr,
    pub dyn_idx: DynIdx,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeValue {
    /// Just a regular shorthand attribute - an ident. Makes our parsing a bit more opaque.
    /// attribute,
    Shorthand(Ident),

    /// Any attribute that's a literal. These get hotreloading super powers
    ///
    /// attribute: "value"
    /// attribute: bool,
    /// attribute: 1,
    AttrLiteral(HotLiteral),

    /// A series of tokens that represent an event handler
    ///
    /// We use a special type here so we can get autocomplete in the closure using partial expansion.
    /// We also do some extra wrapping for improved type hinting since rust sometimes as trouble with
    /// generics and closures.
    EventTokens(PartialClosure),

    /// Unterminated expression - full expressions are handled by AttrExpr
    ///
    /// attribute: if bool { "value" }
    ///
    /// Currently these don't get hotreloading super powers, but they could, depending on how far
    /// we want to go with it
    AttrOptionalExpr {
        condition: Expr,
        value: Box<AttributeValue>,
    },

    /// attribute: some_expr
    /// attribute: {some_expr} ?
    AttrExpr(Expr),
}

impl AttributeValue {
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Shorthand(ident) => ident.span(),
            Self::AttrLiteral(ifmt) => ifmt.span(),
            Self::AttrOptionalExpr { value, .. } => value.span(),
            Self::AttrExpr(expr) => expr.span(),
            Self::EventTokens(closure) => closure.span(),
        }
    }
}

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Shorthand(ident) => ident.to_tokens(tokens),
            Self::AttrLiteral(ifmt) => ifmt.to_tokens(tokens),
            Self::AttrOptionalExpr { condition, value } => {
                tokens.append_all(quote! { if #condition { Some(#value) else { None } } })
            }
            Self::AttrExpr(expr) => expr.to_tokens(tokens),
            Self::EventTokens(closure) => closure.to_tokens(tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn parse_attrs() {
        let _parsed: Attribute = parse2(quote! { name: "value" }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: value }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: "value {fmt}" }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: 123 }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: false }).unwrap();
        let _parsed: Attribute = parse2(quote! { "custom": false }).unwrap();

        // with commas
        let _parsed: Attribute = parse2(quote! { "custom": false, }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: false, }).unwrap();

        // with expressions
        let _parsed: Attribute = parse2(quote! { name: if true { "value" } }).unwrap();
        let _parsed: Attribute =
            parse2(quote! { name: if true { "value" } else { "other" } }).unwrap();

        // with shorthand
        let _parsed: Attribute = parse2(quote! { name }).unwrap();
        let _parsed: Attribute = parse2(quote! { name, }).unwrap();

        // Events - make sure they get partial expansion
        let _parsed: Attribute = parse2(quote! { onclick: |e| {} }).unwrap();
        let _parsed: Attribute = parse2(quote! { onclick: |e| { "value" } }).unwrap();
        let _parsed: Attribute = parse2(quote! { onclick: |e| { value. } }).unwrap();
        let _parsed: Attribute = parse2(quote! { onclick: move |e| { value. } }).unwrap();
        let _parsed: Attribute = parse2(quote! { onclick: move |e| value }).unwrap();
        let _parsed: Attribute = parse2(quote! { onclick: |e| value, }).unwrap();
    }

    #[test]
    fn merge_attrs() {
        let a: Attribute = parse2(quote! { class: "value1" }).unwrap();
        let b: Attribute = parse2(quote! { class: "value2" }).unwrap();

        let b: Attribute = parse2(quote! { class: "value2 {something}" }).unwrap();
        let b: Attribute = parse2(quote! { class: if value { "other thing" } }).unwrap();
        let b: Attribute = parse2(quote! { class: if value { some_expr } }).unwrap();
    }
}
