use proc_macro::TokenStream;
use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, ExprLit, Ident, Lit, LitBool, LitStr, Meta, Path, Token, braced,
    parse_macro_input,
};

#[proc_macro]
pub fn impl_extension_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplExtensionAttributes);
    input.to_token_stream().into()
}

/// Generate typed element constructors and typed attribute extension traits.
#[proc_macro]
pub fn define_elements(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DefineElements);
    input.to_token_stream().into()
}

/// Generate the `EventsExtension` trait that adds event handler methods to typed HTML builders.
///
/// Each entry has the form `#[attrs] method_name => raw_event => DataType,` where `method_name`
/// is the builder method (e.g. `onclick`), `raw_event` is the DOM event name without the `on`
/// prefix (e.g. `click`), and `DataType` is the typed event data (e.g. `MouseData`).
#[proc_macro]
pub fn impl_event_extensions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as EventExtensions);
    input.to_token_stream().into()
}

struct ImplExtensionAttributes {
    name: Ident,
    attrs: Punctuated<ExtensionAttribute, Token![,]>,
    for_el: bool,
    gated_attribute_groups: Vec<GatedAttributeGroup>,
}

struct ExtensionAttribute {
    name: Ident,
    metadata: AttributeMetadata,
}

#[derive(Default)]
struct AttributeMetadata {
    name: Option<LitStr>,
    namespace: Option<LitStr>,
    volatile: bool,
    gated: bool,
}

struct DefineElements {
    core_path: Path,
    html_path: Path,
    context: bool,
    gated_attribute_groups: Vec<GatedAttributeGroup>,
    explicit_gated_attribute_groups: bool,
    elements: Vec<ElementDef>,
}

struct GatedAttributeGroup {
    name: Ident,
    attributes: Punctuated<Ident, Token![,]>,
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

impl Parse for ImplExtensionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let name = input.parse()?;
        braced!(content in input);
        let attrs = content.parse_terminated(ExtensionAttribute::parse, Token![,])?;

        let mut for_el = false;
        let mut gated_attribute_groups = Vec::new();
        while !input.is_empty() {
            let marker: Ident = input.call(Ident::parse_any)?;
            if marker == "for_el" {
                for_el = true;
            } else if marker == "gated_attributes" {
                let content;
                braced!(content in input);
                while !content.is_empty() {
                    gated_attribute_groups.push(content.parse()?);
                    let _ = content.parse::<Token![,]>();
                }
            } else {
                return Err(syn::Error::new(
                    marker.span(),
                    "expected `for_el` or `gated_attributes` after extension attribute list",
                ));
            }
        }

        Ok(ImplExtensionAttributes {
            name,
            attrs,
            for_el,
            gated_attribute_groups,
        })
    }
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
                    Meta::NameValue(name_value)
                        if name_value.path.is_ident("name")
                            || name_value.path.is_ident("rename") =>
                    {
                        metadata.name = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    Meta::NameValue(name_value)
                        if name_value.path.is_ident("namespace")
                            || name_value.path.is_ident("ns") =>
                    {
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
            if let Ok(marker) = fork.call(Ident::parse_any) {
                if marker == "gated_attributes" {
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

impl Parse for GatedAttributeGroup {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.call(Ident::parse_any)?;

        let content;
        braced!(content in input);
        let attributes = content.parse_terminated(Ident::parse_any, Token![,])?;

        Ok(Self { name, attributes })
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
                    Meta::NameValue(name_value)
                        if name_value.path.is_ident("name")
                            || name_value.path.is_ident("rename") =>
                    {
                        metadata.name = Some(lit_str_from_expr(&name_value.value)?);
                    }
                    Meta::NameValue(name_value)
                        if name_value.path.is_ident("namespace")
                            || name_value.path.is_ident("ns") =>
                    {
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

struct EventExtensions {
    events: Vec<EventDef>,
}

struct EventDef {
    attrs: Vec<Attribute>,
    name: Ident,
    raw: Ident,
    data: Ident,
}

impl Parse for EventExtensions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut events = Vec::new();
        while !input.is_empty() {
            let attrs = input.call(Attribute::parse_outer)?;
            let name: Ident = input.call(Ident::parse_any)?;
            input.parse::<Token![=>]>()?;
            let raw: Ident = input.call(Ident::parse_any)?;
            input.parse::<Token![=>]>()?;
            let data: Ident = input.parse()?;
            input.parse::<Token![,]>()?;
            events.push(EventDef {
                attrs,
                name,
                raw,
                data,
            });
        }
        Ok(EventExtensions { events })
    }
}

impl ToTokens for EventExtensions {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let methods = self.events.iter().map(|event| {
            let EventDef {
                attrs,
                name,
                raw,
                data,
            } = event;
            let name_doc = name.to_string();
            let raw_string = raw.to_string();
            let raw_name = raw_string.strip_prefix("r#").unwrap_or(&raw_string);
            let on_name = format!("on{raw_name}");
            // The explicit closure variant is called by the rsx codegen when it sees an inline
            // closure so the closure parameter has a known type without an annotation.
            let explicit_closure = format_ident!("{}_with_explicit_closure", name);

            quote! {
                #[doc = #name_doc]
                #(#attrs)*
                /// <details open>
                /// <summary>General Event Handler Information</summary>
                ///
                #[doc = include_str!("../../docs/event_handlers.md")]
                ///
                /// </details>
                ///
                #[doc = include_str!("../../docs/common_event_handler_errors.md")]
                #[inline]
                fn #name<__Marker>(
                    self,
                    event_handler: impl super::EventHandlerValue<#data, __Marker>,
                ) -> <Self as ::dioxus_core::view::AttributeBuilderTarget>::Output {
                    ::dioxus_core::view::AttributeBuilderTarget::append_attribute(
                        self,
                        super::event_attribute::<#data, __Marker>(#on_name, event_handler),
                    )
                }

                #(#attrs)*
                #[doc(hidden)]
                #[inline]
                fn #explicit_closure<__Marker, __Return>(
                    self,
                    event_handler: impl FnMut(::dioxus_core::Event<#data>) -> __Return + 'static,
                ) -> <Self as ::dioxus_core::view::AttributeBuilderTarget>::Output
                where
                    __Return: ::dioxus_core::SpawnIfAsync<__Marker> + 'static,
                {
                    #[allow(deprecated)]
                    self.#name(event_handler)
                }
            }
        });

        tokens.append_all(quote! {
            /// Event handler extension methods for typed HTML builders.
            pub trait EventsExtension: ::dioxus_core::view::AttributeBuilderTarget + Sized {
                #(#methods)*
            }

            impl<Target> EventsExtension for Target
            where
                Target: ::dioxus_core::view::AttributeBuilderTarget,
            {
            }
        });
    }
}

fn lit_str_from_expr(expr: &Expr) -> syn::Result<LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(lit), ..
        }) => Ok(lit.clone()),
        _ => Err(syn::Error::new_spanned(expr, "expected string literal")),
    }
}

fn ident_to_upper_camel(ident: &Ident) -> String {
    let ident_string = ident.to_string();
    ident_string
        .strip_prefix("r#")
        .unwrap_or(&ident_string)
        .to_case(Case::UpperCamel)
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
        let context = self.context.then(|| self.context_tokens(html));
        let detected_duplicate_macros = (!self.explicit_gated_attribute_groups)
            .then(|| self.detected_duplicate_macro_tokens(gated_attribute_groups));
        let marker_traits = (!self.explicit_gated_attribute_groups)
            .then(|| self.gated_attribute_marker_trait_tokens(gated_attribute_groups));

        tokens.append_all(quote! {
            #detected_duplicate_macros
            #marker_traits
            #(#elements)*
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
                #[doc(hidden)]
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
                let has_attribute = self
                    .attributes
                    .iter()
                    .any(|element_attribute| element_attribute.name == *attribute);

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
        let attrs = self
            .attrs
            .iter()
            .filter(|attr| !attr.path().is_ident("element"));
        let descriptors = self.attributes.iter().map(|attr| {
            let ident = &attr.name;
            let ident_string = ident.to_string();
            let attr_camel_name = ident_string
                .strip_prefix("r#")
                .unwrap_or(&ident_string)
                .to_case(Case::UpperCamel);
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
                #[doc(hidden)]
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
            let ident_string = ident.to_string();
            let attr_camel_name = ident_string
                .strip_prefix("r#")
                .unwrap_or(&ident_string)
                .to_case(Case::UpperCamel);
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
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            pub struct #tag;

            impl #core::view::ElementTag for #tag {
                const NAME: &'static str = #tag_name;
                const NAMESPACE: ::std::option::Option<&'static str> = #namespace;
            }

            #[allow(non_snake_case)]
            #(#attrs)*
            pub const fn #name() -> #core::view::ElementBuilder<#tag, (), ()> {
                #core::view::element_builder::<#tag>()
            }

            impl #attribute_group_marker for #tag {}
            #(#gated_marker_impls)*

            #(#descriptors)*

            pub trait #extension_name: #core::view::AttributeBuilderTarget + Sized {
                #(#methods)*
            }

            impl<__DioxusAttributes, __DioxusChildren> #extension_name
                for #core::view::ElementBuilder<#tag, __DioxusAttributes, __DioxusChildren>
            {
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
        let extension = self.extension_ident();
        quote! {
            pub use super::#extension;
        }
    }
}

impl ExtensionAttribute {
    fn is_gated_by(&self, gated_attributes: &[String]) -> bool {
        self.metadata.gated
            || gated_attributes
                .iter()
                .any(|attr| attr == &self.rust_name())
    }

    fn rust_name(&self) -> String {
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

    fn map_attribute_tokens(&self) -> TokenStream2 {
        let attribute_matches = self.attribute_matches();
        let attribute_name = LitStr::new(&self.attribute_name_value(), self.name.span());
        let namespace = self.namespace_tokens();

        quote! {
            if #attribute_matches {
                return ::std::option::Option::Some((#attribute_name, #namespace));
            }
        }
    }

    fn map_html_attribute_tokens(&self) -> TokenStream2 {
        let html_name = LitStr::new(&self.attribute_name_value(), self.name.span());
        let rsx_name = LitStr::new(&self.rust_name(), self.name.span());

        quote! {
            if html == #html_name {
                return ::std::option::Option::Some(#rsx_name);
            }
        }
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
        let gated_attributes = self
            .gated_attribute_groups
            .iter()
            .find(|group| group.name == *name)
            .map(|group| {
                group
                    .attributes
                    .iter()
                    .map(Ident::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let descriptors = self.attrs.iter().map(|attr| {
            let ident = &attr.name;
            let ident_string = ident.to_string();
            let attr_camel_name = ident_string
                .strip_prefix("r#")
                .unwrap_or(&ident_string)
                .to_case(Case::UpperCamel);
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
                #[doc(hidden)]
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
                let ident_string = ident.to_string();
                let attr_camel_name = ident_string
                    .strip_prefix("r#")
                    .unwrap_or(&ident_string)
                    .to_case(Case::UpperCamel);
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
            }
        });
        let element_impl = self.for_el.then(|| {
            quote! {
                impl<__DioxusAttributes, __DioxusChildren> #extension_name
                    for ::dioxus_core::view::ElementBuilder<#name::Tag, __DioxusAttributes, __DioxusChildren>
                {}
            }
        });
        tokens.append_all(quote! {
            #(#descriptors)*

            pub trait #extension_name: ::dioxus_core::view::AttributeBuilderTarget + Sized {
                #(#impls)*
            }

            #element_impl
            #(#gated_extensions)*
        });
    }
}
