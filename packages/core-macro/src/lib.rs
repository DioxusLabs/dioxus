use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

pub(crate) mod fc;
pub(crate) mod htm;
pub(crate) mod ifmt;
pub(crate) mod props;
pub(crate) mod rsx;
pub(crate) mod rsxtemplate;
pub(crate) mod util;

/// The html! macro makes it easy for developers to write jsx-style markup in their components.
/// We aim to keep functional parity with html templates.
#[proc_macro]
pub fn html(s: TokenStream) -> TokenStream {
    match syn::parse::<htm::HtmlRender>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// The html! macro makes it easy for developers to write jsx-style markup in their components.
/// We aim to keep functional parity with html templates.
#[proc_macro]
pub fn rsx_template(s: TokenStream) -> TokenStream {
    match syn::parse::<rsxtemplate::RsxTemplate>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// The html! macro makes it easy for developers to write jsx-style markup in their components.
/// We aim to keep functional parity with html templates.
#[proc_macro]
pub fn html_template(s: TokenStream) -> TokenStream {
    match syn::parse::<rsxtemplate::RsxTemplate>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

// #[proc_macro_attribute]
// pub fn fc(attr: TokenStream, item: TokenStream) -> TokenStream {

/// Label a function or static closure as a functional component.
/// This macro reduces the need to create a separate properties struct.
///
/// Using this macro is fun and simple
///
/// ```ignore
///
/// #[fc]
/// fn Example(cx: Context, name: &str) -> VNode {
///     cx.render(rsx! { h1 {"hello {name}"} })
/// }
/// ```
#[proc_macro_attribute]
pub fn fc(_attr: TokenStream, item: TokenStream) -> TokenStream {
    match syn::parse::<fc::FunctionComponent>(item) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

#[proc_macro]
pub fn format_args_f(input: TokenStream) -> TokenStream {
    use ifmt::*;
    let item = parse_macro_input!(input as IfmtInput);
    format_args_f_impl(item)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_derive(Props, attributes(builder))]
pub fn derive_typed_builder(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match props::impl_my_derive(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// The html! macro makes it easy for developers to write jsx-style markup in their components.
///
/// ## Complete Reference Guide:
/// ```
/// const Example: FC<()> = |cx| {
///     let formatting = "formatting!";
///     let formatting_tuple = ("a", "b");
///     let lazy_fmt = format_args!("lazily formatted text");
///     cx.render(rsx! {
///         div {
///             // Elements
///             div {}
///             h1 {"Some text"}
///             h1 {"Some text with {formatting}"}
///             h1 {"Formatting basic expressions {formatting_tuple.0} and {formatting_tuple.1}"}
///             h2 {
///                 "Multiple"
///                 "Text"
///                 "Blocks"
///                 "Use comments as separators in html"
///             }
///             div {
///                 h1 {"multiple"}
///                 h2 {"nested"}
///                 h3 {"elements"}
///             }
///             div {
///                 class: "my special div"
///                 h1 {"Headers and attributes!"}
///             }
///             div {
///                 // pass simple rust expressions in
///                 class: lazy_fmt,
///                 id: format_args!("attributes can be passed lazily with std::fmt::Arguments"),
///                 div {
///                     class: {
///                         const WORD: &str = "expressions";
///                         format_args!("Arguments can be passed in through curly braces for complex {}", WORD)
///                     }
///                 }
///             }
///
///             // Expressions can be used in element position too:
///             {rsx!(p { "More templating!" })}
///             {html!(<p>"Even HTML templating!!"</p>)}
///
///             // Iterators
///             {(0..10).map(|i| rsx!(li { "{i}" }))}
///             {{
///                 let data = std::collections::HashMap::<&'static str, &'static str>::new();
///                 // Iterators *should* have keys when you can provide them.
///                 // Keys make your app run faster. Make sure your keys are stable, unique, and predictable.
///                 // Using an "ID" associated with your data is a good idea.
///                 data.into_iter().map(|(k, v)| rsx!(li { key: "{k}" "{v}" }))
///             }}
///            
///             // Matching
///             // Matching will throw a Rust error about "no two closures are the same type"
///             // To fix this, call "render" method or use the "in" syntax to produce VNodes.
///             // There's nothing we can do about it, sorry :/ (unless you want *really* unhygenic macros)
///             {match true {
///                 true => rsx!(in cx, h1 {"Top text"}),
///                 false => cx.render(rsx!( h1 {"Bottom text"}))
///             }}
///
///             // Conditional rendering
///             // Dioxus conditional rendering is based around None/Some. We have no special syntax for conditionals.
///             // You can convert a bool condition to rsx! with .then and .or
///             {true.then(|| rsx!(div {}))}
///
///             // True conditions need to be rendered (same reasons as matching)
///             {if true {
///                 rsx!(in cx, h1 {"Top text"})
///             } else {
///                 rsx!(in cx, h1 {"Bottom text"})
///             }}
///
///             // returning "None" is a bit noisy... but rare in practice
///             {None as Option<()>}
///
///             // Use the Dioxus type-alias for less noise
///             {NONE_ELEMENT}
///
///             // can also just use empty fragments
///             Fragment {}
///
///             // Fragments let you insert groups of nodes without a parent.
///             // This lets you make components that insert elements as siblings without a container.
///             div {"A"}
///             Fragment {
///                 div {"B"}
///                 div {"C"}
///                 Fragment {
///                     "D"
///                     Fragment {
///                         "heavily nested fragments is an antipattern"
///                         "they cause Dioxus to do unnecessary work"
///                         "don't use them carelessly if you can help it"
///                     }
///                 }
///             }
///
///             // Components
///             // Can accept any paths
///             // Notice how you still get syntax highlighting and IDE support :)
///             Baller {}
///             baller::Baller { }
///             crate::baller::Baller {}
///
///             // Can take properties
///             Taller { a: "asd" }
///
///             // Can take optional properties
///             Taller { a: "asd" }
///
///             // Can pass in props directly as an expression
///             {{
///                 let props = TallerProps {a: "hello"};
///                 rsx!(Taller { ..props })
///             }}
///
///             // Spreading can also be overridden manually
///             Taller {
///                 ..TallerProps { a: "ballin!" }
///                 a: "not ballin!"
///             }
///
///             // Can take children too!
///             Taller { a: "asd", div {"hello world!"} }
///         }
///     })
/// };
///
/// mod baller {
///     use super::*;
///     pub struct BallerProps {}
///
///     /// This component totally balls
///     pub fn Baller(cx: Context<()>) -> VNode {
///         todo!()
///     }
/// }
///
/// #[derive(Debug, PartialEq, Props)]
/// pub struct TallerProps {
///     a: &'static str,
/// }
///
/// /// This component is taller than most :)
/// pub fn Taller(cx: Context<TallerProps>) -> VNode {
///     let b = true;
///     todo!()
/// }
/// ```
#[proc_macro]
pub fn rsx(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::RsxRender>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
