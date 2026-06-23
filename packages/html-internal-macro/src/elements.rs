//! Implementation of the `define_elements!` macro, which generates typed element
//! constructors, their attribute extension traits, gated-attribute marker traits,
//! and the optional hot-reload / html-to-rsx context.

use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, LitBool, LitStr, Meta, Path, Token, braced};

use crate::common::{
    ExtensionAttribute, GatedAttributeGroup, ident_to_upper_camel, lit_str_from_expr,
};

pub(crate) struct DefineElements {
    core_path: Path,
    html_path: Path,
    context: bool,
    gated_attribute_groups: Vec<GatedAttributeGroup>,
    explicit_gated_attribute_groups: bool,
    elements: Vec<ElementDef>,
}

struct ElementDef {
    attrs: Vec<Attribute>,
    name: Ident,
    metadata: ElementMetadata,
    attributes: Punctuated<ExtensionAttribute, Token![,]>,
}

#[derive(Default)]
struct ElementMetadata {
    name: Option<LitStr>,
    namespace: Option<LitStr>,
}

impl Parse for DefineElements {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let core_marker: Ident = input.parse()?;
        if core_marker != "core" {
            return Err(syn::Error::new(core_marker.span(), "expected `core`"));
        }
        input.parse::<Token![=]>()?;
        let core_path = input.parse()?;
        input.parse::<Token![,]>()?;

        let html_marker: Ident = input.parse()?;
        if html_marker != "html" {
            return Err(syn::Error::new(html_marker.span(), "expected `html`"));
        }
        input.parse::<Token![=]>()?;
        let html_path = input.parse()?;

        let mut context = false;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Token![;]) {
                break;
            }

            let option: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let enabled: LitBool = input.parse()?;
            if option == "context" {
                context = enabled.value;
            } else {
                return Err(syn::Error::new(
                    option.span(),
                    "expected `context = true` or `context = false`",
                ));
            }
        }

        input.parse::<Token![;]>()?;

        let mut gated_attribute_groups = Vec::new();
        let mut explicit_gated_attribute_groups = false;
        if !input.is_empty() {
            let fork = input.fork();
            if let Ok(marker) = fork.call(Ident::parse_any)
                && marker == "gated_attributes"
            {
                explicit_gated_attribute_groups = true;
                let _marker: Ident = input.call(Ident::parse_any)?;
                let content;
                braced!(content in input);
                while !content.is_empty() {
                    gated_attribute_groups.push(content.parse()?);
                    let _ = content.parse::<Token![,]>();
                }
            }
        }

        let mut elements = Vec::new();
        while !input.is_empty() {
            elements.push(input.parse()?);
        }

        Ok(Self {
            core_path,
            html_path,
            context,
            gated_attribute_groups,
            explicit_gated_attribute_groups,
            elements,
        })
    }
}

impl Parse for ElementDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name = input.parse()?;
        let metadata = ElementMetadata::from_attrs(&attrs)?;

        let content;
        braced!(content in input);
        let attributes = content.parse_terminated(ExtensionAttribute::parse, Token![,])?;

        Ok(Self {
            attrs,
            name,
            metadata,
            attributes,
        })
    }
}

impl ElementMetadata {
    fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut metadata = ElementMetadata::default();

        for attr in attrs {
            if !attr.path().is_ident("element") {
                continue;
            }

            let args = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in args {
                match meta {
                    Meta::NameValue(name_value) if name_value.path.is_ident("name") => {
                        metadata.name = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    Meta::NameValue(name_value) if name_value.path.is_ident("namespace") => {
                        metadata.namespace = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "expected `name = \"...\"` or `namespace = \"...\"`",
                        ));
                    }
                }
            }
        }

        Ok(metadata)
    }
}

impl ToTokens for DefineElements {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let core = &self.core_path;
        let html = &self.html_path;
        let detected_gated_attribute_groups;
        let gated_attribute_groups = if self.explicit_gated_attribute_groups {
            &self.gated_attribute_groups
        } else {
            detected_gated_attribute_groups = self.detected_gated_attribute_groups();
            &detected_gated_attribute_groups
        };
        let elements = self
            .elements
            .iter()
            .map(|element| element.to_tokens_with_paths(core, html, gated_attribute_groups));
        // Each element's associated-const trait and attribute-method extension trait, re-exported
        // anonymously. Glob-importing this one module brings every element into scope so
        // `html::div` resolves and `.class(..)` etc. type-check - the single place downstream
        // preludes (and `define_elements!` callers) pull element names from.
        let prelude_exports = self.elements.iter().map(|element| {
            let name = &element.name;
            let extension = element.extension_ident();
            quote! { pub use super::#name::{#name as _, #extension as _}; }
        });
        let context = self.context.then(|| self.context_tokens(html));
        let detected_duplicate_macros = (!self.explicit_gated_attribute_groups)
            .then(|| self.detected_duplicate_macro_tokens(gated_attribute_groups));
        let marker_traits = (!self.explicit_gated_attribute_groups)
            .then(|| self.gated_attribute_marker_trait_tokens(gated_attribute_groups));

        tokens.append_all(quote! {
            #detected_duplicate_macros
            #marker_traits
            #(#elements)*

            #[allow(unused_imports)]
            pub mod prelude {
                #(#prelude_exports)*
            }

            // Bring the elements into the defining module too, so `define_elements!` callers can
            // use their custom elements in the same module without importing the prelude. This is
            // a private import, so it does not widen the public surface.
            #[allow(unused_imports)]
            use self::prelude::*;
            #context
        });
    }
}

impl DefineElements {
    fn detected_gated_attribute_groups(&self) -> Vec<GatedAttributeGroup> {
        let mut groups = BTreeMap::<&'static str, BTreeMap<String, Ident>>::new();

        for element in &self.elements {
            let group = element.attribute_group_name();
            let attributes = groups.entry(group).or_default();
            for attribute in &element.attributes {
                attributes
                    .entry(attribute.rust_name())
                    .or_insert_with(|| attribute.name.clone());
            }
        }

        groups
            .into_iter()
            .map(|(group, attributes)| GatedAttributeGroup {
                name: Ident::new(group, proc_macro2::Span::call_site()),
                attributes: attributes.into_values().collect(),
            })
            .collect()
    }

    fn detected_duplicate_macro_tokens(
        &self,
        gated_attribute_groups: &[GatedAttributeGroup],
    ) -> TokenStream2 {
        let group_blocks = gated_attribute_groups.iter().map(|group| {
            let name = &group.name;
            let attributes = group.attributes.iter();
            quote! {
                #name { #(#attributes,)* }
            }
        });
        let define_group_blocks = group_blocks.clone();

        quote! {
            #[doc(hidden)]
            #[macro_export]
            macro_rules! __dioxus_html_define_elements_with_detected_gated_attributes {
                (
                    $target_macro:path,
                    core = $core:path,
                    html = $html:path
                    $(, $option:ident = $enabled:tt)* $(,)?
                    ;
                    $($body:tt)*
                ) => {
                    $target_macro! {
                        core = $core,
                        html = $html
                        $(, $option = $enabled)*;
                        gated_attributes {
                            #(#define_group_blocks)*
                        }
                        $($body)*
                    }
                };
            }

            #[doc(hidden)]
            #[macro_export]
            macro_rules! __dioxus_html_impl_extension_attributes_with_detected_gated_attributes {
                (
                    $target_macro:path,
                    $name:ident { $($attrs:tt)* }
                    $($rest:tt)*
                ) => {
                    $target_macro![
                        $name { $($attrs)* }
                        gated_attributes {
                            #(#group_blocks)*
                        }
                        $($rest)*
                    ];
                };
            }
        }
    }

    fn gated_attribute_marker_trait_tokens(
        &self,
        gated_attribute_groups: &[GatedAttributeGroup],
    ) -> TokenStream2 {
        let markers = gated_attribute_groups.iter().flat_map(|group| {
            let group_camel_name = ident_to_upper_camel(&group.name);
            group.attributes.iter().map(move |attribute| {
                let attr_camel_name = ident_to_upper_camel(attribute);
                Ident::new(
                    format!("{group_camel_name}{attr_camel_name}Element").as_str(),
                    attribute.span(),
                )
            })
        });

        quote! {
            #(
                /// Marker trait for elements that support a gated attribute.
                pub trait #markers {}
            )*
        }
    }

    fn context_tokens(&self, html: &Path) -> TokenStream2 {
        let map_attribute = self
            .elements
            .iter()
            .map(|element| element.map_attribute_tokens(html));
        let map_element = self.elements.iter().map(ElementDef::map_element_tokens);
        let map_html_attribute = self
            .elements
            .iter()
            .flat_map(|element| element.attributes.iter())
            .map(ExtensionAttribute::map_html_attribute_tokens);
        let map_html_element = self
            .elements
            .iter()
            .map(ElementDef::map_html_element_tokens);
        let extension_exports = self
            .elements
            .iter()
            .map(ElementDef::extension_export_tokens);

        quote! {
            #[cfg(feature = "hot-reload-context")]
            pub struct HtmlCtx;

            #[cfg(feature = "hot-reload-context")]
            impl ::dioxus_core_types::HotReloadingContext for HtmlCtx {
                fn map_attribute(
                    element: &str,
                    attribute: &str,
                ) -> ::std::option::Option<(&'static str, ::std::option::Option<&'static str>)> {
                    #(#map_attribute)*
                    ::std::option::Option::None
                }

                fn map_element(
                    element: &str,
                ) -> ::std::option::Option<(&'static str, ::std::option::Option<&'static str>)> {
                    #(#map_element)*
                    ::std::option::Option::None
                }
            }

            #[cfg(feature = "html-to-rsx")]
            pub fn map_html_attribute_to_rsx(html: &str) -> ::std::option::Option<&'static str> {
                #(#map_html_attribute)*

                if let ::std::option::Option::Some(name) = #html::map_html_global_attributes_to_rsx(html) {
                    return ::std::option::Option::Some(name);
                }

                if let ::std::option::Option::Some(name) = #html::map_html_svg_attributes_to_rsx(html) {
                    return ::std::option::Option::Some(name);
                }

                ::std::option::Option::None
            }

            #[cfg(feature = "html-to-rsx")]
            pub fn map_html_element_to_rsx(html: &str) -> ::std::option::Option<&'static str> {
                #(#map_html_element)*
                ::std::option::Option::None
            }

            #[allow(unused_imports)]
            pub(crate) mod extensions {
                #(#extension_exports)*
            }
        }
    }
}

impl ElementDef {
    fn to_tokens_with_paths(
        &self,
        core: &Path,
        html: &Path,
        gated_attribute_groups: &[GatedAttributeGroup],
    ) -> TokenStream2 {
        let name = &self.name;
        let name_string = self.rust_name();
        let camel_name = self.camel_name();
        let tag = Ident::new(format!("{camel_name}Element").as_str(), name.span());
        let extension_name = self.extension_ident();
        let spread_marker = self.spread_marker_ident();
        let tag_name = self
            .metadata
            .name
            .as_ref()
            .map(|name| quote! { #name })
            .unwrap_or_else(|| {
                let ident = name_string.strip_prefix("r#").unwrap_or(&name_string);
                quote! { #ident }
            });
        let namespace = self
            .metadata
            .namespace
            .as_ref()
            .map(|namespace| quote! { ::std::option::Option::Some(#namespace) })
            .unwrap_or_else(|| quote! { ::std::option::Option::None });
        let attribute_group_marker = match self.namespace_value().as_deref() {
            Some("http://www.w3.org/2000/svg") => quote! { #html::SvgAttributesElement },
            _ => quote! { #html::GlobalAttributesElement },
        };
        let attribute_group_name = self.attribute_group_name();
        let mut gated_marker_impls = Vec::new();
        for group in gated_attribute_groups
            .iter()
            .filter(|group| group.name == attribute_group_name)
        {
            let group_camel_name = ident_to_upper_camel(&group.name);
            for attribute in &group.attributes {
                let attribute_name = attribute.to_string();
                let has_attribute = self
                    .attributes
                    .iter()
                    .any(|element_attribute| element_attribute.rust_name() == attribute_name);

                if !has_attribute {
                    let attr_camel_name = ident_to_upper_camel(attribute);
                    let marker = Ident::new(
                        format!("{group_camel_name}{attr_camel_name}Element").as_str(),
                        attribute.span(),
                    );

                    gated_marker_impls.push(quote! { impl #html::#marker for #tag {} });
                }
            }
        }
        // Element doc comments, reused on both the trait const and the impl const, so
        // collect into a reusable token stream rather than a single-use iterator.
        let attrs = self
            .attrs
            .iter()
            .filter(|attr| !attr.path().is_ident("element"))
            .map(|attr| quote! { #attr })
            .collect::<TokenStream2>();
        let descriptors = self.attributes.iter().map(|attr| {
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
                pub struct #descriptor;

                impl #core::view::AttributeDescriptor for #descriptor {
                    const NAME: &'static str = #attr_name;
                    const NAMESPACE: ::std::option::Option<&'static str> = #namespace;
                    const VOLATILE: bool = #volatile;
                }
            }
        });
        let methods = self.attributes.iter().map(|attr| {
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
                ) -> <__DioxusAttributeValue as #core::view::IntoAttributeBuilderValue<
                    Self,
                    #descriptor,
                    __DioxusAttributeMarker,
                >>::Output
                where
                    __DioxusAttributeValue: #core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttributeMarker,
                    >,
                {
                    <__DioxusAttributeValue as #core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttributeMarker,
                    >>::append_to(value, self)
                }
            }
        });

        quote! {
            // One public module per element. It holds the element's tag marker, attribute
            // descriptors, the element associated-const trait, the attribute-method extension
            // trait, and the spread marker. The element trait is brought into scope through the
            // generated `prelude` module (anonymously) and the extension/spread traits through
            // `extensions`; the tag and descriptor markers stay reachable only by path here.
            #[allow(non_snake_case, non_camel_case_types)]
            pub mod #name {
                // Zero-information markers for the tag and its attributes. `pub` only so they
                // can appear in this module's public trait signatures.
                #[allow(non_camel_case_types)]
                pub struct #tag;

                impl #core::view::ElementTag for #tag {
                    const NAME: &'static str = #tag_name;
                    const NAMESPACE: ::std::option::Option<&'static str> = #namespace;
                }

                impl #attribute_group_marker for #tag {}
                #(#gated_marker_impls)*

                #(#descriptors)*

                /// Per-element trait carrying the element as an associated const on the
                /// shared `html` root, so `html::#name` resolves wherever this trait is
                /// in scope. Other vocabularies extend the same root the same way.
                #[allow(non_camel_case_types, non_upper_case_globals)]
                pub trait #name {
                    #attrs
                    const #name: #core::view::ElementBuilder<#tag, (), ()>;
                }

                #[allow(non_upper_case_globals)]
                impl #name for #html::html {
                    #attrs
                    const #name: #core::view::ElementBuilder<#tag, (), ()> =
                        #core::view::element_builder::<#tag>();
                }

                pub trait #extension_name: #core::view::AttributeBuilderTarget + Sized {
                    #(#methods)*
                }

                impl<__DioxusAttributes, __DioxusChildren> #extension_name
                    for #core::view::ElementBuilder<#tag, __DioxusAttributes, __DioxusChildren>
                {
                }

                /// Marker for catch-all attribute targets (e.g. `#[props(extends = ...)]`
                /// spread builders) that accept this element's attributes. Implementing it
                /// grants the element's attribute methods.
                pub trait #spread_marker {}

                impl<__DioxusSpreadTarget> #extension_name for __DioxusSpreadTarget
                where
                    __DioxusSpreadTarget:
                        #spread_marker + #core::view::AttributeBuilderTarget,
                {
                }
            }
        }
    }
}

impl ElementDef {
    fn rust_name(&self) -> String {
        self.name.to_string()
    }

    fn rsx_name(&self) -> String {
        self.rust_name()
            .strip_prefix("r#")
            .map(ToString::to_string)
            .unwrap_or_else(|| self.rust_name())
    }

    fn tag_name_value(&self) -> String {
        self.metadata
            .name
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_else(|| self.rsx_name())
    }

    fn namespace_value(&self) -> Option<String> {
        self.metadata.namespace.as_ref().map(LitStr::value)
    }

    fn attribute_group_name(&self) -> &'static str {
        match self.namespace_value().as_deref() {
            Some("http://www.w3.org/2000/svg") => "svg_attributes",
            _ => "global_attributes",
        }
    }

    fn camel_name(&self) -> String {
        self.rsx_name().to_case(Case::UpperCamel)
    }

    fn extension_ident(&self) -> Ident {
        Ident::new(
            format!("{}Extension", self.camel_name()).as_str(),
            self.name.span(),
        )
    }

    /// Marker that lets `#[props(extends = ...)]` spread builders opt into this element's
    /// attributes. Element extensions are not gated, so it is an empty marker; it exists so
    /// the props macro can reference one uniformly across element and group extensions.
    fn spread_marker_ident(&self) -> Ident {
        Ident::new(
            format!("{}SpreadTarget", self.camel_name()).as_str(),
            self.name.span(),
        )
    }

    fn element_matches(&self) -> TokenStream2 {
        let rust_name = LitStr::new(&self.rust_name(), self.name.span());
        let rsx_name = LitStr::new(&self.rsx_name(), self.name.span());

        if rust_name.value() == rsx_name.value() {
            quote! { element == #rust_name }
        } else {
            quote! { element == #rust_name || element == #rsx_name }
        }
    }

    fn namespace_tokens(&self) -> TokenStream2 {
        self.metadata
            .namespace
            .as_ref()
            .map(|namespace| quote! { ::std::option::Option::Some(#namespace) })
            .unwrap_or_else(|| quote! { ::std::option::Option::None })
    }

    fn map_attribute_tokens(&self, html: &Path) -> TokenStream2 {
        let element_matches = self.element_matches();
        let attributes = self
            .attributes
            .iter()
            .map(ExtensionAttribute::map_attribute_tokens);
        let fallback = match self.namespace_value().as_deref() {
            Some("http://www.w3.org/2000/svg") => quote! { #html::map_svg_attributes(attribute) },
            _ => quote! { #html::map_global_attributes(attribute) },
        };

        quote! {
            if #element_matches {
                #(#attributes)*
                return #fallback;
            }
        }
    }

    fn map_element_tokens(&self) -> TokenStream2 {
        let element_matches = self.element_matches();
        let tag_name = LitStr::new(&self.tag_name_value(), self.name.span());
        let namespace = self.namespace_tokens();

        quote! {
            if #element_matches {
                return ::std::option::Option::Some((#tag_name, #namespace));
            }
        }
    }

    fn map_html_element_tokens(&self) -> TokenStream2 {
        let html_name = LitStr::new(&self.tag_name_value(), self.name.span());
        let rsx_name = LitStr::new(&self.rsx_name(), self.name.span());

        quote! {
            if html == #html_name {
                return ::std::option::Option::Some(#rsx_name);
            }
        }
    }

    fn extension_export_tokens(&self) -> TokenStream2 {
        let name = &self.name;
        let extension = self.extension_ident();
        let spread_marker = self.spread_marker_ident();
        quote! {
            pub use super::#name::#extension;
            pub use super::#name::#spread_marker;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn element_metadata_rejects_name_aliases() {
        let rename = syn::parse2::<ElementDef>(quote! {
            #[element(rename = "custom-element")]
            customElement {}
        });
        assert!(rename.is_err());

        let ns = syn::parse2::<ElementDef>(quote! {
            #[element(ns = "test")]
            customElement {}
        });
        assert!(ns.is_err());
    }
}
