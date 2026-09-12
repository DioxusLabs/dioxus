//! Implementation of the `impl_extension_attributes!` macro, which generates an
//! attribute extension trait (and the gated-attribute extensions) for a group of
//! attributes such as the global or SVG attribute sets.

use std::path::Path;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::Item;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, braced};

use crate::common::{ExtensionAttribute, ident_to_upper_camel};

pub(crate) struct ImplExtensionAttributes {
    name: Ident,
    attrs: Punctuated<ExtensionAttribute, Token![,]>,
}

impl Parse for ImplExtensionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let name = input.parse()?;
        braced!(content in input);
        let attrs = content.parse_terminated(ExtensionAttribute::parse, Token![,])?;

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after extension attribute list"));
        }

        Ok(ImplExtensionAttributes { name, attrs })
    }
}

impl ToTokens for ImplExtensionAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let name_string = name.to_string();
        let camel_name = name_string
            .strip_prefix("r#")
            .unwrap_or(&name_string)
            .to_case(Case::UpperCamel);
        let extension_name = Ident::new(format!("{}Extension", &camel_name).as_str(), name.span());
        let group_marker = Ident::new(format!("{camel_name}Element").as_str(), name.span());
        // Marker for catch-all attribute targets (e.g. `#[props(extends = ...)]` spread
        // builders) that accept every attribute in this group, gated ones included.
        let spread_marker = Ident::new(format!("{camel_name}SpreadTarget").as_str(), name.span());
        let gated_attributes = detect_builtin_gated_attributes_for(&name_string);

        let descriptors = self.attrs.iter().map(|attr| {
            let ident = &attr.name;
            let ident_string = ident.to_string();
            let attr_camel_name = ident_to_upper_camel(ident);
            let descriptor = Ident::new(
                format!("{camel_name}{attr_camel_name}AttributeDescriptor").as_str(),
                ident.span(),
            );
            let attr_name = attr
                .metadata
                .name
                .as_ref()
                .map(|name| quote! { #name })
                .unwrap_or_else(|| {
                    let ident = ident_string.strip_prefix("r#").unwrap_or(&ident_string);
                    quote! { #ident }
                });
            let namespace = attr
                .metadata
                .namespace
                .as_ref()
                .map(|namespace| quote! { ::std::option::Option::Some(#namespace) })
                .unwrap_or_else(|| quote! { ::std::option::Option::None });
            let volatile = attr.metadata.volatile;
            quote! {
                /// Static metadata for this generated attribute.
                pub struct #descriptor;

                impl ::dioxus_core::view::AttributeDescriptor for #descriptor {
                    const NAME: &'static str = #attr_name;
                    const NAMESPACE: ::std::option::Option<&'static str> = #namespace;
                    const VOLATILE: bool = #volatile;
                }
            }
        });

        let impls = self
            .attrs
            .iter()
            .filter(|attr| !attr.is_gated_by(&gated_attributes))
            .map(|attr| {
                let ident = &attr.name;
                let attr_camel_name = ident_to_upper_camel(ident);
                let descriptor = Ident::new(
                    format!("{camel_name}{attr_camel_name}AttributeDescriptor").as_str(),
                    ident.span(),
                );
                quote! {
                    #[allow(non_snake_case)]
                    fn #ident<__DioxusAttributeMarker, __DioxusAttributeValue>(
                        self,
                        value: __DioxusAttributeValue,
                    ) -> <__DioxusAttributeValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttributeMarker,
                    >>::Output
                    where
                        __DioxusAttributeValue: ::dioxus_core::view::IntoAttributeBuilderValue<
                            Self,
                            #descriptor,
                            __DioxusAttributeMarker,
                        >,
                    {
                        <__DioxusAttributeValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                            Self,
                            #descriptor,
                            __DioxusAttributeMarker,
                        >>::append_to(value, self)
                    }
                }
            });
        let gated_extensions = self.attrs.iter().filter(|attr| attr.is_gated_by(&gated_attributes)).map(|attr| {
            let ident = &attr.name;
            let attr_camel_name = ident_to_upper_camel(ident);
            let descriptor = Ident::new(
                format!("{camel_name}{attr_camel_name}AttributeDescriptor").as_str(),
                ident.span(),
            );
            let extension_name = Ident::new(
                format!("{camel_name}{attr_camel_name}Extension").as_str(),
                ident.span(),
            );
            let marker = Ident::new(
                format!("{camel_name}{attr_camel_name}Element").as_str(),
                ident.span(),
            );

            quote! {
                pub trait #extension_name: ::dioxus_core::view::AttributeBuilderTarget + Sized {
                    #[allow(non_snake_case)]
                    fn #ident<__DioxusAttributeMarker, __DioxusAttributeValue>(
                        self,
                        value: __DioxusAttributeValue,
                    ) -> <__DioxusAttributeValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttributeMarker,
                    >>::Output
                    where
                        __DioxusAttributeValue: ::dioxus_core::view::IntoAttributeBuilderValue<
                            Self,
                            #descriptor,
                            __DioxusAttributeMarker,
                        >,
                    {
                        <__DioxusAttributeValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                            Self,
                            #descriptor,
                            __DioxusAttributeMarker,
                        >>::append_to(value, self)
                    }
                }

                impl<__DioxusTag, __DioxusAttributes, __DioxusChildren> #extension_name
                    for ::dioxus_core::view::ElementBuilder<
                        __DioxusTag,
                        __DioxusAttributes,
                        __DioxusChildren,
                    >
                where
                    __DioxusTag: #group_marker + crate::#marker,
                {
                }

                // Spread targets accept every attribute in the group, so they get
                // gated attributes unconditionally (no per-element marker required).
                impl<__DioxusSpreadTarget> #extension_name for __DioxusSpreadTarget
                where
                    __DioxusSpreadTarget:
                        crate::#spread_marker + ::dioxus_core::view::AttributeBuilderTarget,
                {
                }
            }
        });
        tokens.append_all(quote! {
            #(#descriptors)*

            /// Marker for catch-all attribute targets that accept every attribute in this
            /// group, including gated ones. A `#[props(extends = ...)]` spread builder
            /// implements this marker to receive the group's full attribute extension methods.
            pub trait #spread_marker {}

            pub trait #extension_name: ::dioxus_core::view::AttributeBuilderTarget + Sized {
                #(#impls)*
            }

            // Spread targets accept every attribute in the group, so route the (non-gated)
            // umbrella extension through the marker as well.
            impl<__DioxusSpreadTarget> #extension_name for __DioxusSpreadTarget
            where
                __DioxusSpreadTarget:
                    #spread_marker + ::dioxus_core::view::AttributeBuilderTarget,
            {
            }

            #(#gated_extensions)*
        });
    }
}

fn detect_builtin_gated_attributes_for(group: &str) -> Vec<String> {
    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return Vec::new();
    };
    let elements_path = Path::new(&manifest_dir).join("src").join("elements.rs");
    let Ok(elements_source) = std::fs::read_to_string(elements_path) else {
        return Vec::new();
    };
    let Ok(elements_file) = syn::parse_file(&elements_source) else {
        return Vec::new();
    };

    for item in elements_file.items {
        let Item::Macro(item_macro) = item else {
            continue;
        };
        let Some(segment) = item_macro.mac.path.segments.last() else {
            continue;
        };
        if segment.ident != "define_elements" {
            continue;
        }
        let Ok(elements) = syn::parse2::<crate::elements::DefineElements>(item_macro.mac.tokens)
        else {
            continue;
        };

        return elements.detected_gated_attribute_names_for(group);
    }

    Vec::new()
}
