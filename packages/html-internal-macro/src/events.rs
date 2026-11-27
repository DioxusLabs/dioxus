//! Implementation of the `impl_event_extensions!` macro, which generates the
//! `EventsExtension` trait that adds typed event handler methods to HTML
//! builders.

use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Token, bracketed};

pub(crate) struct EventExtensions {
    events: Vec<EventDef>,
}

/// The objects an event can be listened for on, from `#[targets(...)]` in the event table.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Targets {
    element: bool,
    document: bool,
    window: bool,
}

impl Targets {
    const ALL: Self = Self {
        element: true,
        document: true,
        window: true,
    };

    fn parse(attr: &Attribute) -> syn::Result<Self> {
        let mut targets = Self {
            element: false,
            document: false,
            window: false,
        };
        for target in attr.parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated)? {
            match target.to_string().as_str() {
                "element" => targets.element = true,
                "document" => targets.document = true,
                "window" => targets.window = true,
                _ => {
                    return Err(syn::Error::new(
                        target.span(),
                        "expected `element`, `document` or `window`",
                    ));
                }
            }
        }
        Ok(targets)
    }
}

struct EventDef {
    attrs: Vec<Attribute>,
    name: Ident,
    raw: Ident,
    data: Ident,
    targets: Targets,
}

impl Parse for EventExtensions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut events = Vec::new();
        // Each group is an optional `#[targets(...)]` followed by its bracketed events.
        while !input.is_empty() {
            let mut targets = Targets::ALL;
            for attr in input.call(Attribute::parse_outer)? {
                if !attr.path().is_ident("targets") {
                    return Err(syn::Error::new_spanned(attr, "expected `#[targets(...)]`"));
                }
                targets = Targets::parse(&attr)?;
            }
            let group;
            bracketed!(group in input);
            while !group.is_empty() {
                let attrs = group.call(Attribute::parse_outer)?;
                let name: Ident = group.call(Ident::parse_any)?;
                group.parse::<Token![=>]>()?;
                let raw: Ident = group.call(Ident::parse_any)?;
                group.parse::<Token![=>]>()?;
                let data: Ident = group.parse()?;
                group.parse::<Token![,]>()?;
                events.push(EventDef {
                    attrs,
                    name,
                    raw,
                    data,
                    targets,
                });
            }
        }
        Ok(EventExtensions { events })
    }
}

impl ToTokens for EventExtensions {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let methods = self.events.iter().filter(|event| event.targets.element).map(|event| {
            let EventDef {
                attrs,
                name,
                raw,
                data,
                ..
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
                ) -> <Self as ::dioxus_vdom::view::AttributeBuilderTarget>::Output {
                    ::dioxus_vdom::view::AttributeBuilderTarget::append_attribute(
                        self,
                        super::event_attribute::<#data, __Marker>(#on_name, event_handler),
                    )
                }

                #(#attrs)*
                #[doc(hidden)]
                #[inline]
                fn #explicit_closure<__Marker, __Return>(
                    self,
                    event_handler: impl FnMut(::dioxus_vdom::Event<#data>) -> __Return + 'static,
                ) -> <Self as ::dioxus_vdom::view::AttributeBuilderTarget>::Output
                where
                    __Return: ::dioxus_vdom::SpawnIfAsync<__Marker> + 'static,
                {
                    #[allow(deprecated)]
                    self.#name(event_handler)
                }
            }
        });

        let shared = global_methods(&self.events, |targets| targets.document && targets.window);
        let window = global_methods(&self.events, |targets| targets.window && !targets.document);
        let document = global_methods(&self.events, |targets| targets.document && !targets.window);

        tokens.append_all(quote! {
            /// Event handler extension methods for typed HTML builders.
            pub trait EventsExtension: ::dioxus_vdom::view::AttributeBuilderTarget + Sized {
                #(#methods)*
            }

            impl<Target> EventsExtension for Target
            where
                Target: ::dioxus_vdom::view::AttributeBuilderTarget,
            {
            }

            /// Events that can be listened for on both the window and the document.
            ///
            /// The listener macros look events up as methods of this trait and of the
            /// per-target [`WindowEventsExtension`] and [`DocumentEventsExtension`], so
            /// they must be in scope where the macros are used. The prelude exports them.
            pub trait GlobalEventsExtension: GlobalEventTarget {
                #(#shared)*
            }

            impl<Target: GlobalEventTarget> GlobalEventsExtension for Target {}

            /// Events that can only be listened for on the window.
            pub trait WindowEventsExtension: GlobalEventTarget {
                #(#window)*
            }

            impl WindowEventsExtension for WindowTarget {}

            /// Events that can only be listened for on the document.
            pub trait DocumentEventsExtension: GlobalEventTarget {
                #(#document)*
            }

            impl DocumentEventsExtension for DocumentTarget {}
        });
    }
}

/// One method per event name whose targets satisfy `include`, returning the event paired with
/// its payload type.
fn global_methods(events: &[EventDef], include: impl Fn(Targets) -> bool) -> Vec<TokenStream2> {
    // Several element events can share a DOM name (`ondblclick` and `ondoubleclick`).
    let mut by_raw: HashMap<String, usize> = HashMap::new();
    let mut selected: Vec<&EventDef> = Vec::new();
    let mut errors = Vec::new();
    for event in events.iter().filter(|event| include(event.targets)) {
        match by_raw.get(&event.raw.to_string()) {
            Some(&index) if selected[index].data != event.data => errors.push(
                syn::Error::new(
                    event.raw.span(),
                    "events with the same name and target must share a payload type",
                )
                .to_compile_error(),
            ),
            Some(&index) => {
                if is_deprecated(selected[index]) && !is_deprecated(event) {
                    selected[index] = event;
                }
            }
            None => {
                by_raw.insert(event.raw.to_string(), selected.len());
                selected.push(event);
            }
        }
    }

    selected
        .into_iter()
        .map(|event| {
            let EventDef {
                attrs, raw, data, ..
            } = event;
            let raw_name = raw.unraw().to_string();
            let docs = attrs.iter().filter(|attr| attr.path().is_ident("doc"));
            let summary = format!("The `{raw_name}` event.");
            quote! {
                #[doc = #summary]
                ///
                #(#docs)*
                #[inline]
                fn #raw(self) -> GlobalEvent<Self, #data> {
                    GlobalEvent::new(#raw_name)
                }
            }
        })
        .chain(errors)
        .collect()
}

fn is_deprecated(event: &EventDef) -> bool {
    event
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("deprecated"))
}
