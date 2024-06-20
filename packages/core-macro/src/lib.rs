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

/// Format a string with inline rust expressions. [`format_args_f!`] is very similar to [`format_args`], but it allows you to use arbitrary rust expressions inside braces instead of just variables:
///
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// let formatted_with_variables = format_args!("{} + {} = {}", 1, 2, 1 + 2);
/// let formatted_with_inline_expressions = format_args_f!("{1} + {2} = {1 + 2}");
/// ```
#[proc_macro]
pub fn format_args_f(input: TokenStream) -> TokenStream {
    use rsx::*;
    parse_macro_input!(input as IfmtInput)
        .into_token_stream()
        .into()
}

#[doc = include_str!("../docs/props.md")]
#[proc_macro_derive(Props, attributes(props))]
pub fn derive_typed_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match props::impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[doc = include_str!("../docs/rsx.md")]
#[proc_macro]
pub fn rsx(tokens: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(tokens) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.into_token_stream().into(),
    }
}

/// The rsx! macro makes it easy for developers to write jsx-style markup in their components.
#[deprecated(note = "Use `rsx!` instead.")]
#[proc_macro]
pub fn render(tokens: TokenStream) -> TokenStream {
    rsx(tokens)
}

#[doc = include_str!("../docs/component.md")]
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
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// #[inline_props]
/// fn GreetBob(bob: String) -> Element {
///     rsx! { "hello, {bob}" }
/// }
/// ```
///
/// is equivalent to
///
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// #[derive(PartialEq, Props, Clone)]
/// struct AppProps {
///     bob: String,
/// }
///
/// fn GreetBob(props: AppProps) -> Element {
///     rsx! { "hello, {props.bob}" }
/// }
/// ```
#[proc_macro_attribute]
#[deprecated(note = "Use `#[component]` instead.")]
pub fn inline_props(args: TokenStream, input: TokenStream) -> TokenStream {
    component(args, input)
}
