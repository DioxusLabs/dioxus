#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

use component::ComponentBody;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod component;
mod props;
mod utils;

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

/// Streamlines component creation.
/// This is the recommended way of creating components,
/// though you might want lower-level control with more advanced uses.
///
/// # Arguments
/// * `no_case_check` - Doesn't enforce `PascalCase` on your component names.
/// **This will be removed/deprecated in a future update in favor of a more complete Clippy-backed linting system.**
/// The reasoning behind this is that Clippy allows more robust and powerful lints, whereas
/// macros are extremely limited.
///
/// # Features
/// This attribute:
/// * Enforces that your component uses `PascalCase`.
/// No warnings are generated for the `PascalCase`
/// function name, but everything else will still raise a warning if it's incorrectly `PascalCase`.
/// Does not disable warnings anywhere else, so if you, for example,
/// accidentally don't use `snake_case`
/// for a variable name in the function, the compiler will still warn you.
/// * Automatically uses `#[inline_props]` if there's more than 1 parameter in the function.
/// * Verifies the validity of your component.
///
/// # Examples
/// * Without props:
/// ```rust,ignore
/// #[component]
/// fn GreetBob() -> Element {
///     rsx! { "hello, bob" }
/// }
/// ```
///
/// * With props:
/// ```rust,ignore
/// #[component]
/// fn GreetBob(bob: String) -> Element {
///    rsx! { "hello, {bob}" }
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
