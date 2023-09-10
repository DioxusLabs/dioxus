use proc_macro::TokenStream;
use quote::ToTokens;
use rsx::RenderCallBody;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Path, Token};

mod component_body;
mod component_body_deserializers;
mod props;

// mod rsx;
use crate::component_body::ComponentBody;
use crate::component_body_deserializers::component::ComponentDeserializerArgs;
use crate::component_body_deserializers::inline_props::InlinePropsDeserializerArgs;
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
pub fn rsx(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => body.to_token_stream().into(),
    }
}

/// The render! macro makes it easy for developers to write jsx-style markup in their components.
///
/// The render macro automatically renders rsx - making it unhygienic.
#[proc_macro]
pub fn render(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => RenderCallBody(body).into_token_stream().into(),
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
/// ```rust,ignore
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
pub fn inline_props(_args: TokenStream, s: TokenStream) -> TokenStream {
    let comp_body = parse_macro_input!(s as ComponentBody);

    match comp_body.deserialize(InlinePropsDeserializerArgs {}) {
        Err(e) => e.to_compile_error().into(),
        Ok(output) => output.to_token_stream().into(),
    }
}

pub(crate) const COMPONENT_ARG_CASE_CHECK_OFF: &str = "no_case_check";

/// Streamlines component creation.
/// This is the recommended way of creating components,
/// though you might want lower-level control with more advanced uses.
///
/// # Arguments
/// * `no_case_check` - Turns off `snake_case` checking and doesn't convert names to `PascalCase`.
/// If you're following coding conventions, you should not need to use this.
/// However, it is necessary,
/// because the macro actually *errors* when faced with a non `snake_case` name,
/// since it's impossible for procedural macros to give warnings.
///
/// # Features
/// This attribute:
/// * Renames your `snake_case` function to use `PascalCase`,
/// without generating any warnings.
/// Does not disable warnings anywhere else, so if you, for example,
/// accidentally don't use `snake_case`
/// for a variable name in the function, the compiler will still warn you.
/// * Automatically uses `#[inline_props]` if there's more than 1 parameter in the function.
/// * Verifies the validity of your component.
/// E.g. if it has a [`Scope`](dioxus_core::Scope) argument.
/// Notes:
///     * This doesn't work 100% of the time, because of macro limitations.
///     * Provides helpful messages if your component is not correct.
/// Possible bugs (please, report these!):
///     * There might be bugs where it incorrectly *denies* validity.
/// This is bad as it means that you can't use the attribute or you have to change the component.
///     * There might be bugs where it incorrectly *confirms* validity.
/// You will still know if the component is invalid once you use it,
/// but the error might be less helpful.
///
/// # Examples
/// * Without props:
/// ```rust,ignore
/// #[component]
/// fn greet_bob(cx: Scope) -> Element {
///     render! { "hello, bob" }
/// }
///
/// // is equivalent to
///
/// #[allow(non_snake_case)]
/// fn GreetBob(cx: Scope) -> Element {
///     // There's no function call overhead since __greet_bob has the #[inline(always)] attribute,
///     // so don't worry about performance.
///     __greet_bob(cx)
/// }
///
/// #[inline(always)]
/// fn __greet_bob(cx: Scope) -> Element {
///     render! { "hello, bob" }
/// }
/// ```
/// * With props and the `no_case_check` argument:
/// ```rust,ignore
/// #[component(no_case_check)]
/// fn GREET_PERSON(cx: Scope, person: String) -> Element {
///     render! { "hello, {person}" }
/// }
///
/// // is equivalent to
///
/// #[derive(Props, PartialEq)]
/// #[allow(non_camel_case_types)]
/// struct GREET_PERSONProps {
///     person: String,
/// }
///
/// #[allow(non_snake_case)]
/// fn GREET_PERSON<'a>(cx: Scope<'a, GREET_PERSONProps>) -> Element {
///     __greet_person(cx)
/// }
///
/// #[inline(always)]
/// fn __GREET_PERSON<'a>(cx: Scope<'a, GREET_PERSONProps>) -> Element {
///     let GREET_PERSONProps { person } = &cx.props;
///     {
///         render! { "hello, {person}" }
///     }
/// }
/// ```
// TODO: Maybe add an option to input a custom component name through the args.
//  I think that's unnecessary, but there might be some scenario where it could be useful.
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    let component_body = parse_macro_input!(input as ComponentBody);
    let case_check = match Punctuated::<Path, Token![,]>::parse_terminated.parse(args) {
        Err(e) => return e.to_compile_error().into(),
        Ok(args) => {
            if let Some(first) = args.first() {
                !first.is_ident(COMPONENT_ARG_CASE_CHECK_OFF)
            } else {
                true
            }
        }
    };

    match component_body.deserialize(ComponentDeserializerArgs { case_check }) {
        Err(e) => e.to_compile_error().into(),
        Ok(output) => output.to_token_stream().into(),
    }
}
