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
    let item = parse_macro_input!(input as IfmtInput);
    format_args_f_impl(item)
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
///
/// ## Complete Reference Guide:
/// ```
#[doc = include_str!("../../../examples/rsx_usage.rs")]
/// ```
#[proc_macro]
pub fn rsx(s: TokenStream) -> TokenStream {
    #[cfg(feature = "hot-reload")]
    let rsx_text = s.to_string();
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => {
            #[cfg(feature = "hot-reload")]
            {
                use dioxus_rsx_interpreter::captuered_context::CapturedContextBuilder;

                match CapturedContextBuilder::from_call_body(body) {
                    Ok(captured) => {
                        let lazy = quote::quote! {
                            LazyNodes::new(move |__cx|{
                                let code_location = get_line_num!();
                                let captured = #captured;
                                let text = #rsx_text;

                                resolve_scope(code_location, text, captured, __cx)
                            })
                        };
                        if let Some(cx) = captured.custom_context {
                            quote::quote! {
                                #cx.render(#lazy)
                            }
                            .into()
                        } else {
                            lazy.into()
                        }
                    }
                    Err(err) => err.into_compile_error().into(),
                }
            }
            #[cfg(not(feature = "hot-reload"))]
            body.to_token_stream().into()
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
/// ```
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
