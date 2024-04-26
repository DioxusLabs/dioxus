#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use component::ComponentBody;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod component;
mod props;

use dioxus_rsx as rsx;

#[proc_macro]
pub fn format_args_f(input: TokenStream) -> TokenStream {
    use rsx::*;
    format_args_f_impl(parse_macro_input!(input as IfmtInput))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Props, attributes(props))]
pub fn derive_typed_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match props::impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The `rsx!` macro makes it easy for developers to write jsx-style markup in their components.
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(tokens) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.into_token_stream().into(),
    }
}

/// This macro has been deprecated in favor of [`rsx`].
#[deprecated(note = "Use `rsx!` instead.")]
#[proc_macro]
pub fn render(tokens: TokenStream) -> TokenStream {
    rsx(tokens)
}

/// * Makes the compiler allow an `UpperCamelCase` function identifier.
/// * Seamlessly creates a props struct if there's more than 1 parameter in the function.
/// * Verifies the validity of your component.
///
/// # Examples
///
/// * Without props:
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn Greet() -> Element {
///     rsx! { "hello, someone" }
/// }
/// ```
///
/// * With props:
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn Greet(person: String) -> Element {
///    rsx! { "hello, " {person} }
/// }
/// ```
/// Which is roughly equivalent to:
/// ```rust
/// # use dioxus::prelude::*;
/// #[derive(PartialEq, Clone, Props)]
/// struct GreetProps {
///     person: String,
/// }
///
/// fn Greet(GreetProps { person }: GreetProps) -> Element {
///     rsx! { "hello, " {person} }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_args: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ComponentBody)
        .into_token_stream()
        .into()
}

/// This macro has been deprecated in favor of [`component`].
#[proc_macro_attribute]
#[deprecated(note = "Use `#[component]` instead.")]
pub fn inline_props(args: TokenStream, input: TokenStream) -> TokenStream {
    component(args, input)
}
