use proc_macro::TokenStream;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Expr, ExprLit, Ident, Lit, LitStr, Meta, Token, braced, parenthesized,
    parse_macro_input,
};

#[proc_macro]
pub fn impl_extension_attributes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplExtensionAttributes);
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
}

impl Parse for ImplExtensionAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;

        let name = input.parse()?;
        braced!(content in input);
        let attrs = content.parse_terminated(ExtensionAttribute::parse, Token![,])?;
        let for_el = if input.is_empty() {
            false
        } else {
            let marker: Ident = input.parse()?;
            if marker != "for_el" {
                return Err(syn::Error::new(
                    marker.span(),
                    "expected `for_el` after extension attribute list",
                ));
            }
            true
        };

        Ok(ImplExtensionAttributes {
            name,
            attrs,
            for_el,
        })
    }
}

impl Parse for ExtensionAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name = input.parse()?;
        let mut metadata = AttributeMetadata::from_attrs(&attrs)?;
        metadata.merge(AttributeMetadata::parse_legacy(input)?);

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
                    Meta::Path(path) if path.is_ident("no_alias") => {}
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
                            "expected `volatile`, `no_alias`, `name = \"...\"`, or `namespace = \"...\"`",
                        ));
                    }
                }
            }
        }

        Ok(metadata)
    }

    fn merge(&mut self, other: AttributeMetadata) {
        if other.name.is_some() {
            self.name = other.name;
        }
        if other.namespace.is_some() {
            self.namespace = other.namespace;
        }
        self.volatile |= other.volatile;
    }

    fn parse_legacy(input: ParseStream) -> syn::Result<Self> {
        let mut metadata = AttributeMetadata::default();

        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            if input.peek(Ident::peek_any) {
                let maybe_no_alias: Ident = input.call(Ident::parse_any)?;
                if maybe_no_alias != "no" {
                    return Err(syn::Error::new(
                        maybe_no_alias.span(),
                        "expected attribute alias string",
                    ));
                }
                input.parse::<Token![-]>()?;
                let alias: Ident = input.call(Ident::parse_any)?;
                if alias != "alias" {
                    return Err(syn::Error::new(alias.span(), "expected `alias`"));
                }
            }
            metadata.name = Some(input.parse()?);
        }

        if input.peek(Ident::peek_any) {
            let marker: Ident = input.call(Ident::parse_any)?;
            match marker.to_string().as_str() {
                "volatile" => metadata.volatile = true,
                "in" => metadata.namespace = Some(input.parse()?),
                _ => {
                    return Err(syn::Error::new(
                        marker.span(),
                        "expected `volatile` or `in`",
                    ));
                }
            }
        } else if input.peek(LitStr) {
            metadata.name = Some(input.parse()?);
        } else if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            if content.peek(Ident::peek_any) {
                let marker: Ident = content.call(Ident::parse_any)?;
                match marker.to_string().as_str() {
                    "volatile" => metadata.volatile = true,
                    "in" => {
                        metadata.namespace = Some(content.parse()?);
                        if content.peek(Token![:]) {
                            content.parse::<Token![:]>()?;
                            let volatile: Ident = content.call(Ident::parse_any)?;
                            if volatile != "volatile" {
                                return Err(syn::Error::new(
                                    volatile.span(),
                                    "expected `volatile`",
                                ));
                            }
                            metadata.volatile = true;
                        }
                    }
                    _ => {
                        return Err(syn::Error::new(
                            marker.span(),
                            "expected `volatile` or `in`",
                        ));
                    }
                }
            } else if content.peek(LitStr) {
                metadata.name = Some(content.parse()?);
            }

            if !content.is_empty() {
                return Err(content.error("unexpected attribute metadata"));
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
                ) -> <Self as ::dioxus_core::view::AttributeTarget>::Output {
                    ::dioxus_core::view::AttributeTarget::append_attribute(
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
                ) -> <Self as ::dioxus_core::view::AttributeTarget>::Output
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
            pub trait EventsExtension: ::dioxus_core::view::AttributeTarget + Sized {
                #(#methods)*
            }

            impl<Target> EventsExtension for Target
            where
                Target: ::dioxus_core::view::AttributeTarget,
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

impl ToTokens for ImplExtensionAttributes {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let name_string = name.to_string();
        let camel_name = name_string
            .strip_prefix("r#")
            .unwrap_or(&name_string)
            .to_case(Case::UpperCamel);
        let extension_name = Ident::new(format!("{}Extension", &camel_name).as_str(), name.span());

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
                .map(|namespace| quote! { Some(#namespace) })
                .unwrap_or_else(|| quote! { None });
            let volatile = attr.metadata.volatile;
            quote! {
                #[doc(hidden)]
                pub struct #descriptor;

                impl ::dioxus_core::view::AttributeDescriptor for #descriptor {
                    const NAME: &'static str = #attr_name;
                    const NAMESPACE: ::dioxus_core::TemplateRawAttrNamespace = #namespace;
                    const VOLATILE: bool = #volatile;
                }
            }
        });

        let impls = self.attrs.iter().map(|attr| {
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
                fn #ident<__DioxusAttrMarker, __DioxusAttrValue>(
                    self,
                    value: __DioxusAttrValue,
                ) -> <__DioxusAttrValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                    Self,
                    #descriptor,
                    __DioxusAttrMarker,
                >>::Output
                where
                    __DioxusAttrValue: ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttrMarker,
                    >,
                {
                    <__DioxusAttrValue as ::dioxus_core::view::IntoAttributeBuilderValue<
                        Self,
                        #descriptor,
                        __DioxusAttrMarker,
                    >>::append_to(value, self)
                }
            }
        });
        let element_impl = self.for_el.then(|| {
            quote! {
                impl<__DioxusAttrs, __DioxusChildren> #extension_name
                    for ::dioxus_core::view::El<#name::Tag, __DioxusAttrs, __DioxusChildren>
                {}
            }
        });
        tokens.append_all(quote! {
            #(#descriptors)*

            pub trait #extension_name: ::dioxus_core::view::AttributeTarget + Sized {
                #(#impls)*
            }

            #element_impl
        });
    }
}
