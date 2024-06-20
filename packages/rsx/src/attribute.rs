use std::fmt::Display;

use crate::{innerlude::*, HotReloadingContext};
use dioxus_core::prelude::TemplateAttribute;
use proc_macro2::{Literal, TokenStream};
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

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Attribute {
    pub name: AttributeName,
    pub value: AttributeValue,
    pub dyn_idx: DynIdx,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeName {
    BuiltIn(Ident),
    Custom(LitStr),
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
}

impl ToTokens for AttributeName {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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

    pub fn rendered_as_dynamic_attr(&self, el_name: &ElementName) -> TokenStream {
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
