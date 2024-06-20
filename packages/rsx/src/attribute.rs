use std::fmt::Display;

use crate::{innerlude::*, HotReloadingContext};
use dioxus_core::prelude::TemplateAttribute;
use proc_macro2::{Literal, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseBuffer},
    spanned::Spanned,
    token::{self, Brace},
    AngleBracketedGenericArguments, Expr, ExprClosure, ExprIf, Ident, Lit, LitStr, PatLit,
    PathArguments, Token,
};

use super::literal::RsxLiteral;

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
    AttrLiteral(RsxLiteral),

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
    AttrExpr(Expr),
}

// ..spread attribute
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Spread {
    pub dots: Token![..],
    pub expr: Expr,
    pub dyn_idx: DynIdx,
}

impl Display for AttributeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(lit) => write!(f, "{}", lit.value()),
            Self::BuiltIn(ident) => write!(f, "{}", ident),
        }
    }
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

impl ToTokens for AttributeName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Custom(lit) => lit.to_tokens(tokens),
            Self::BuiltIn(ident) => ident.to_tokens(tokens),
        }
    }
}

impl AttributeValue {
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Shorthand(ident) => ident.span(),
            Self::AttrLiteral(ifmt) => ifmt.span(),
            Self::AttrOptionalExpr { value, .. } => value.span(),
            Self::AttrExpr(expr) => expr.span(),
        }
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
                    (HotLiteral::Fmted(a), HotLiteral::Fmted(b)) => {
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

    pub fn as_lit(&self) -> Option<&RsxLiteral> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => Some(lit),
            _ => None,
        }
    }

    /// Run this closure against the attribute if it's hotreloadable
    pub fn with_hr(&self, f: impl FnOnce(&RsxLiteral)) {
        if let AttributeValue::AttrLiteral(ifmt) = &self.value {
            f(ifmt);
        }
    }

    pub fn ifmt(&self) -> Option<&IfmtInput> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => match &lit.value {
                HotLiteral::Fmted(input) => Some(input),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_static_str_literal(&self) -> Option<(&AttributeName, &IfmtInput)> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => match &lit.value {
                HotLiteral::Fmted(input) if input.is_static() => Some((&self.name, input)),
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

        let attribute = |name: &AttributeName| match name {
            AttributeName::BuiltIn(name) => match el_name {
                ElementName::Ident(_) => quote! { dioxus_elements::#el_name::#name.0 },
                ElementName::Custom(_) => {
                    let as_string = name.to_string();
                    quote!(#as_string)
                }
            },
            AttributeName::Custom(s) => quote! { #s },
        };

        let value = &self.value;
        let name = &self.name;

        let is_event = match &self.name {
            AttributeName::BuiltIn(name) => name.to_string().starts_with("on"),
            _ => false,
        };

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
        if is_event {
            quote! {
                Box::new([
                    dioxus_elements::events::#name(#value)
                ])
            }
        } else {
            let ns = ns(name);
            let volatile = volatile(name);
            let attribute = attribute(name);
            let value = quote! { #value };

            quote! {
                Box::new([
                    dioxus_core::Attribute::new(
                        #attribute,
                        #value,
                        #ns,
                        #volatile
                    )
                ])
            }
        }
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

    pub(crate) fn try_combine(&self, other: &Self) -> Option<Self> {
        if self.name == other.name {
            if let Some(separator) = self.name.multi_attribute_separator() {
                todo!()
                // return Some(ElementAttrNamed {
                //     el_name: self.el_name.clone(),
                //     attr: ElementAttr {
                //         name: self.attr.name.clone(),
                //         value: self.attr.value.combine(separator, &other.attr.value),
                //     },
                //     followed_by_comma: self.followed_by_comma || other.followed_by_comma,
                // });
            }
        }

        None
    }

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

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Shorthand(ident) => ident.to_tokens(tokens),
            Self::AttrLiteral(ifmt) => ifmt.to_tokens(tokens),
            Self::AttrOptionalExpr { condition, value } => {
                tokens.append_all(quote! { if #condition { Some(#value) else { None } } })
            }
            Self::AttrExpr(expr) => expr.to_tokens(tokens),
        }
    }
}
