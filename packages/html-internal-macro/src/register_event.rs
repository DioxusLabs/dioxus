//! Implementation of the `register_event!` macro, which gives a custom event dispatched in the
//! page a typed handler that rsx can attach like any built-in event.

use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, TokenStreamExt, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, ExprLit, Ident, Lit, LitBool, LitStr, Token};

pub(crate) struct RegisterEvent {
    krate: TokenStream2,
    attrs: Vec<Attribute>,
    name: Ident,
    js_event: Expr,
    bubbles: LitBool,
}

fn keyword(input: ParseStream, word: &str) -> syn::Result<()> {
    let found: Ident = input.parse()?;
    if found != word {
        return Err(syn::Error::new(
            found.span(),
            format!("expected `{word}`, found `{found}`"),
        ));
    }
    Ok(())
}

impl Parse for RegisterEvent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut krate = TokenStream2::new();
        while !input.peek(Token![;]) {
            krate.append(input.parse::<TokenTree>()?);
        }
        input.parse::<Token![;]>()?;

        let attrs = input.call(Attribute::parse_outer)?;

        keyword(input, "event")?;
        input.parse::<Token![:]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        keyword(input, "js")?;
        input.parse::<Token![-]>()?;
        keyword(input, "event")?;
        input.parse::<Token![:]>()?;
        let js_event: Expr = input.parse()?;
        input.parse::<Token![;]>()?;

        keyword(input, "bubbles")?;
        input.parse::<Token![:]>()?;
        let bubbles: LitBool = input.parse()?;
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        }

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after `bubbles`"));
        }
        if !name.to_string().starts_with("on") || name == "on" {
            return Err(syn::Error::new(
                name.span(),
                "a custom event handler is named `on` followed by the event, so that rsx attaches it as an event",
            ));
        }
        if let Some(raw) = literal(&js_event)
            && (raw.value().is_empty() || raw.value().chars().any(char::is_whitespace))
        {
            return Err(syn::Error::new(
                raw.span(),
                "a custom event is dispatched under a name with no whitespace",
            ));
        }

        Ok(Self {
            krate,
            attrs,
            name,
            js_event,
            bubbles,
        })
    }
}

fn literal(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(raw), ..
        }) => Some(raw),
        _ => None,
    }
}

impl ToTokens for RegisterEvent {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            krate,
            attrs,
            name,
            js_event,
            bubbles,
        } = self;
        let listener = match literal(js_event) {
            Some(raw) => {
                let listener = LitStr::new(&format!("on{}", raw.value()), raw.span());
                quote! { #listener }
            }
            None => quote! {{
                static LISTENER: ::std::sync::OnceLock<&'static str> = ::std::sync::OnceLock::new();
                *LISTENER.get_or_init(|| {
                    ::std::boxed::Box::leak(::std::format!("on{}", #js_event).into_boxed_str())
                })
            }},
        };
        let explicit_closure = format_ident!("{}_with_explicit_closure", name);

        tokens.append_all(quote! {
            #(#attrs)*
            #[allow(non_camel_case_types)]
            pub trait #name:
                #krate::dioxus_vdom::view::AttributeBuilderTarget + ::core::marker::Sized
            {
                #(#attrs)*
                #[inline]
                fn #name<__Marker>(
                    self,
                    event_handler: impl #krate::events::EventHandlerValue<#krate::events::CustomData, __Marker>,
                ) -> <Self as #krate::dioxus_vdom::view::AttributeBuilderTarget>::Output {
                    #krate::events::register_custom_event(#js_event, #bubbles);
                    #krate::dioxus_vdom::view::AttributeBuilderTarget::append_attribute(
                        self,
                        #krate::events::custom_event_attribute::<__Marker>(#listener, event_handler),
                    )
                }

                #[doc(hidden)]
                #[inline]
                fn #explicit_closure<__Marker, __Return>(
                    self,
                    event_handler: impl ::core::ops::FnMut(
                        #krate::dioxus_vdom::Event<#krate::events::CustomData>,
                    ) -> __Return + 'static,
                ) -> <Self as #krate::dioxus_vdom::view::AttributeBuilderTarget>::Output
                where
                    __Return: #krate::dioxus_vdom::SpawnIfAsync<__Marker> + 'static,
                {
                    self.#name(event_handler)
                }
            }

            impl<__Target> #name for __Target
            where
                __Target: #krate::dioxus_vdom::view::AttributeBuilderTarget,
            {
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(tokens: TokenStream2) -> syn::Result<RegisterEvent> {
        syn::parse2(tokens)
    }

    fn refused(tokens: TokenStream2) -> String {
        match parsed(tokens) {
            Ok(_) => panic!("The registration was read."),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn a_registration_under_a_literal_is_read_and_listens_under_its_name() {
        let registration = parsed(quote! {
            dioxus_html;
            /// Says something.
            event: onnotice;
            js-event: "notice";
            bubbles: true;
        })
        .expect("The registration is read.");
        assert_eq!(registration.name, "onnotice");
        assert_eq!(registration.attrs.len(), 1);
        assert!(registration.bubbles.value);
        let written = registration.to_token_stream().to_string();
        assert!(written.contains("\"onnotice\""), "{written}");
        assert!(!written.contains("OnceLock"), "{written}");
        assert!(
            written.contains("onnotice_with_explicit_closure"),
            "{written}"
        );
    }

    #[test]
    fn a_registration_under_a_constant_names_its_listener_as_it_runs() {
        let registration = parsed(quote! {
            dioxus_html;
            event: onnotice;
            js-event: crate::NOTICE;
            bubbles: false
        })
        .expect("The registration is read, with or without its last semicolon.");
        assert!(!registration.bubbles.value);
        let written = registration.to_token_stream().to_string();
        assert!(written.contains("OnceLock"), "{written}");
        assert!(written.contains("crate :: NOTICE"), "{written}");
    }

    #[test]
    fn a_handler_not_named_for_an_event_is_refused() {
        for name in [quote! { notice }, quote! { on }] {
            assert!(
                refused(quote! { dioxus_html; event: #name; js-event: "notice"; bubbles: true; })
                    .contains("named `on` followed by the event")
            );
        }
    }

    #[test]
    fn an_event_name_that_is_empty_or_holds_whitespace_is_refused() {
        for name in ["", "two words"] {
            assert!(
                refused(quote! { dioxus_html; event: onnotice; js-event: #name; bubbles: true; })
                    .contains("no whitespace")
            );
        }
    }

    #[test]
    fn a_misspelled_or_missing_key_is_refused() {
        assert!(
            refused(quote! { dioxus_html; evnt: onnotice; js-event: "notice"; bubbles: true; })
                .contains("expected `event`")
        );
        assert!(
            refused(quote! { dioxus_html; event: onnotice; js-name: "notice"; bubbles: true; })
                .contains("expected `event`")
        );
        assert!(
            refused(quote! { dioxus_html; event: onnotice; js-event: "notice"; bubbling: true; })
                .contains("expected `bubbles`")
        );
        assert!(!refused(quote! { dioxus_html; event: onnotice; js-event: "notice"; }).is_empty());
    }

    #[test]
    fn anything_after_the_registration_is_refused() {
        assert!(
            refused(
                quote! { dioxus_html; event: onnotice; js-event: "notice"; bubbles: true; extra }
            )
            .contains("unexpected tokens")
        );
    }
}
