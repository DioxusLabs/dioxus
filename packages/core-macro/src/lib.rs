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

/// The rsx! macro makes it easy for developers to write jsx-style markup in their components.
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(tokens) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.into_token_stream().into(),
    }
}

/// The rsx! macro makes it easy for developers to write jsx-style markup in their components.
///
/// The render macro automatically renders rsx - making it unhygienic.
#[deprecated(note = "Use `rsx!` instead.")]
#[proc_macro]
pub fn render(tokens: TokenStream) -> TokenStream {
    rsx(tokens)
}

/// * Silences warnings for the `PascalCase` function name.
/// * Seamlessly creates a props struct if there's more than 1 parameter in the function.
/// * Verifies the validity of your component.
///
/// # Examples
/// 
/// * Without props:
/// ```rust
/// #[component]
/// fn Greet() -> Element {
///     rsx! { "hello, someone" }
/// }
/// ```
///
/// * With props:
/// ```rust
/// #[component]
/// fn Greet(person: String) -> Element {
///    rsx! { "hello, {person}" }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_args: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ComponentBody)
        .into_token_stream()
        .into()
}

/// Derive props for a component within the component definition.
///
/// This macro provides a simple transformation from `Scope<{}>` to `Scope<P>`,
/// removing some boilerplate when defining props.
///
/// You don't *need* to use this macro at all, but it can be helpful in cases where
/// you would be repeating a lot of the usual Rust boilerplate.
///
/// # Example
/// ```rust,ignore
/// #[inline_props]
/// fn app(bob: String) -> Element {
///     rsx! { "hello, {bob}") }
/// }
///
/// // is equivalent to
///
/// #[derive(PartialEq, Props)]
/// struct AppProps {
///     bob: String,
/// }
///
/// fn app(props: AppProps) -> Element {
///     rsx! { "hello, {bob}") }
/// }
/// ```
#[proc_macro_attribute]
#[deprecated(note = "Use `#[component]` instead.")]
pub fn inline_props(args: TokenStream, input: TokenStream) -> TokenStream {
    component(args, input)
}
