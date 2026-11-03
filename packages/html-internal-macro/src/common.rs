//! Types and helpers shared across the macro implementations: the attribute
//! definitions parsed from element/group bodies, gated attribute groups, and
//! small token/string utilities.

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprLit, Ident, Lit, LitStr, Meta, Token, braced};

/// A single attribute entry inside an element or attribute-group body, e.g.
/// `#[attr(name = "data-foo", volatile)] data_foo`.
pub(crate) struct ExtensionAttribute {
    pub(crate) name: Ident,
    pub(crate) metadata: AttributeMetadata,
}

#[derive(Default)]
pub(crate) struct AttributeMetadata {
    pub(crate) name: Option<LitStr>,
    pub(crate) namespace: Option<LitStr>,
    pub(crate) volatile: bool,
    pub(crate) gated: bool,
}

/// A named set of attributes that are gated behind a per-element marker trait.
pub(crate) struct GatedAttributeGroup {
    pub(crate) name: Ident,
    pub(crate) attributes: Punctuated<Ident, Token![,]>,
}

impl Parse for ExtensionAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name = input.parse()?;
        let metadata = AttributeMetadata::from_attrs(&attrs)?;

        Ok(Self { name, metadata })
    }
}

impl AttributeMetadata {
    fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut metadata = AttributeMetadata::default();

        for attr in attrs {
            if !attr.path().is_ident("attr") {
                continue;
            }

            let args = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in args {
                match meta {
                    Meta::Path(path) if path.is_ident("volatile") => {
                        metadata.volatile = true;
                    }
                    Meta::Path(path) if path.is_ident("gated") => {
                        metadata.gated = true;
                    }
                    Meta::NameValue(name_value) if name_value.path.is_ident("name") => {
                        metadata.name = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    Meta::NameValue(name_value) if name_value.path.is_ident("namespace") => {
                        metadata.namespace = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "expected `volatile`, `gated`, `name = \"...\"`, or `namespace = \"...\"`",
                        ));
                    }
                }
            }
        }

        Ok(metadata)
    }
}

impl Parse for GatedAttributeGroup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.call(Ident::parse_any)?;

        let content;
        braced!(content in input);
        let attributes = content.parse_terminated(Ident::parse_any, Token![,])?;

        Ok(Self { name, attributes })
    }
}

impl ExtensionAttribute {
    pub(crate) fn is_gated_by(&self, gated_attributes: &[String]) -> bool {
        self.metadata.gated
            || gated_attributes
                .iter()
                .any(|attr| attr == &self.rust_name())
    }

    pub(crate) fn rust_name(&self) -> String {
        self.name.to_string()
    }

    fn rsx_name(&self) -> String {
        self.rust_name()
            .strip_prefix("r#")
            .map(ToString::to_string)
            .unwrap_or_else(|| self.rust_name())
    }

    fn attribute_name_value(&self) -> String {
        self.metadata
            .name
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_else(|| self.rsx_name())
    }

    fn namespace_tokens(&self) -> TokenStream2 {
        self.metadata
            .namespace
            .as_ref()
            .map(|namespace| quote! { ::std::option::Option::Some(#namespace) })
            .unwrap_or_else(|| quote! { ::std::option::Option::None })
    }

    fn attribute_matches(&self) -> TokenStream2 {
        let rust_name = LitStr::new(&self.rust_name(), self.name.span());
        let rsx_name = LitStr::new(&self.rsx_name(), self.name.span());

        if rust_name.value() == rsx_name.value() {
            quote! { attribute == #rust_name }
        } else {
            quote! { attribute == #rust_name || attribute == #rsx_name }
        }
    }

    pub(crate) fn map_attribute_tokens(&self) -> TokenStream2 {
        let attribute_matches = self.attribute_matches();
        let attribute_name = LitStr::new(&self.attribute_name_value(), self.name.span());
        let namespace = self.namespace_tokens();

        quote! {
            if #attribute_matches {
                return ::std::option::Option::Some((#attribute_name, #namespace));
            }
        }
    }

    pub(crate) fn map_html_attribute_tokens(&self) -> TokenStream2 {
        let html_name = LitStr::new(&self.attribute_name_value(), self.name.span());
        let rsx_name = LitStr::new(&self.rust_name(), self.name.span());

        quote! {
            if html == #html_name {
                return ::std::option::Option::Some(#rsx_name);
            }
        }
    }
}

pub(crate) fn lit_str_from_expr(expr: &Expr) -> syn::Result<LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.clone()),
        _ => Err(syn::Error::new_spanned(expr, "expected string literal")),
    }
}

pub(crate) fn ident_to_upper_camel(ident: &Ident) -> String {
    let ident_string = ident.to_string();
    ident_string
        .strip_prefix("r#")
        .unwrap_or(&ident_string)
        .to_case(Case::UpperCamel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn attr_metadata_rejects_name_aliases() {
        let rename = syn::parse2::<ExtensionAttribute>(quote! {
            #[attr(rename = "data-test")]
            data_test
        });
        assert!(rename.is_err());

        let ns = syn::parse2::<ExtensionAttribute>(quote! {
            #[attr(ns = "test")]
            data_test
        });
        assert!(ns.is_err());
    }
}
