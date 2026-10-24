//! Implementation of the `impl_event_extensions!` macro, which generates the
//! `EventsExtension` trait that adds typed event handler methods to HTML
//! builders.

use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Token};

pub(crate) struct EventExtensions {
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
