use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod inlineprops;
mod props;

// mod rsx;
use dioxus_rsx as rsx;

#[proc_macro]
pub fn format_args_f(input: TokenStream) -> TokenStream {
    use rsx::*;
    format_args_f_impl(parse_macro_input!(input as IfmtInput))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Props, attributes(props))]
pub fn derive_typed_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match props::impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The rsx! macro makes it easy for developers to write jsx-style markup in their components.
#[proc_macro]
pub fn rsx(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.to_token_stream().into(),
    }
}

/// The render! macro makes it easy for developers to write jsx-style markup in their components.
///
/// The render macro automatically renders rsx - making it unhygenic.
#[proc_macro]
pub fn render(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(mut body) => {
            body.inline_cx = true;
            body.into_token_stream().into()
        }
    }
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
/// ```ignore
/// #[inline_props]
/// fn app(cx: Scope, bob: String) -> Element {
///     cx.render(rsx!("hello, {bob}"))
/// }
///
/// // is equivalent to
///
/// #[derive(PartialEq, Props)]
/// struct AppProps {
///     bob: String,
/// }
///
/// fn app(cx: Scope<AppProps>) -> Element {
///     cx.render(rsx!("hello, {bob}"))
/// }
/// ```
#[proc_macro_attribute]
pub fn inline_props(_args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match syn::parse::<inlineprops::InlinePropsBody>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
