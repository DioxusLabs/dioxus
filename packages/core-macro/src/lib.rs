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
///
/// ## Elements
///
/// You can render elements with rsx! with the element name and then braces surrounding the attributes and children.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx! {
///     div {
///         div {}
///     }
/// };
/// ```
///
/// <details>
/// <summary>Web Components</summary>
///
///
/// Dioxus will automatically render any elements with `-` as a untyped web component:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx! {
///     div-component {
///         div {}
///     }
/// };
/// ```
///
/// You can wrap your web component in a custom component to add type checking:
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// #[component]
/// fn MyDivComponent(width: i64) -> Element {
///     rsx! {
///         div-component {
///             "width": width
///         }
///     }
/// }
/// ```
///
///
/// </details>
///
/// ## Attributes
///
/// You can add attributes to any element inside the braces. Attributes are key-value pairs separated by a colon.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// let width = 100;
/// rsx! {
///     div {
///         // Set the class attribute to "my-class"
///         class: "my-class",
///         // attribute strings are automatically formatted with the format macro
///         width: "{width}px",
///     }
/// };
/// ```
///
/// ### Optional Attributes
///
/// You can include optional attributes with an unterminated if statement as the value of the attribute:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # let first_boolean = true;
/// # let second_boolean = false;
/// rsx! {
///     div {
///         // Set the class attribute to "my-class" if true
///         class: if first_boolean {
///             "my-class"
///         },
///         // Set the class attribute to "my-other-class" if false
///         class: if second_boolean {
///             "my-other-class"
///         }
///     }
/// };
/// ```
///
/// ### Raw Attributes
///
/// Dioxus defaults to attributes that are type checked as html. If you want to include an attribute that is not included in the html spec, you can use the `raw` attribute surrounded by quotes:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx! {
///     div {
///         // Set the data-count attribute to "1"
///         "data-count": "1"
///     }
/// };
/// ```
///
/// ## Text
///
/// You can include text in your markup as a string literal:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// let name = "World";
/// rsx! {
///     div {
///         "Hello World"
///         // Just like attributes, you can included formatted segments inside your text
///         "Hello {name}"
///     }
/// };
/// ```
///
/// ## Components
///
/// You can render any [`macro@crate::component`]s you created inside your markup just like elements. Components must either start with a capital letter or contain a `_` character.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// #[component]
/// fn HelloWorld() -> Element {
///     rsx! { "hello world!" }
/// }
///
/// rsx! {
///     div {
///         HelloWorld {}
///     }
/// };
/// ```
///
/// ## If statements
///
/// You can use if statements to conditionally render children. The body of the for if statement is parsed as rsx markup:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// let first_boolean = true;
/// let second_boolean = false;
/// rsx! {
///     if first_boolean {
///         div {
///             "first"
///         }
///     }
///
///     if second_boolean {
///         "second"
///     }
/// };
/// ```
///
/// ## For loops
///
/// You can also use for loops to iterate over a collection of items. The body of the for loop is parsed as rsx markup:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// let numbers = vec![1, 2, 3];
/// rsx! {
///     for number in numbers {
///         div {
///             "{number}"
///         }
///     }
/// };
/// ```
///
/// ## Raw Expressions
///
/// You can include raw expressions inside your markup inside curly braces. Your expression must implement the [`IntoDynNode`](https://docs.rs/dioxus-core/latest/dioxus_core/trait.IntoDynNode.html) trait:
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// let name = "World";
/// rsx! {
///     div {
///         // Text can be converted into a dynamic node in rsx
///         {name}
///     }
///     // Iterators can also be converted into dynamic nodes
///     {(0..10).map(|n| n * n).map(|number| rsx! { div { "{number}" } })}
/// };
/// ```
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
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// #[component]
/// fn GreetBob() -> Element {
///     rsx! { "hello, bob" }
/// }
/// ```
///
/// * With props:
/// ```rust, no_run
/// # use dioxus::prelude::*;
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
