use proc_macro::TokenStream;
use quote::{quote, ToTokens};
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
/// const Example: Component = |cx| {
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
///             {match true {
///                 true => rsx!(h1 {"Top text"}),
///                 false => rsx!(h1 {"Bottom text"})
///             }}
///
///             // Conditional rendering
///             // Dioxus conditional rendering is based around None/Some. We have no special syntax for conditionals.
///             // You can convert a bool condition to rsx! with .then and .or
///             {true.then(|| rsx!(div {}))}
///
///             // True conditions
///             {if true {
///                 rsx!(h1 {"Top text"})
///             } else {
///                 rsx!(h1 {"Bottom text"})
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
///     pub fn Baller(cx: Scope) -> DomTree {
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
/// pub fn Taller(cx: Scope<TallerProps>) -> DomTree {
///     let b = true;
///     todo!()
/// }
/// ```
#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn rsx(s: TokenStream) -> TokenStream {
    #[cfg(feature = "hot_reload")]
    let rsx_text = s.to_string();
    match syn::parse::<rsx::CallBody>(s) {
        Err(err) => err.to_compile_error().into(),
        Ok(body) => {
            #[cfg(feature = "hot_reload")]
            {
                use dioxus_rsx_interperter::captuered_context::CapturedContextBuilder;

                let captured = CapturedContextBuilder::from_call_body(body);
                quote! {
                    {
                        let __line_num = get_line_num();
                        let __rsx_text_index: RsxTextIndex = cx.consume_context().unwrap();
                        // only the insert the rsx text once
                        if !__rsx_text_index.read().contains_key(&__line_num){
                            __rsx_text_index.insert(
                                __line_num.clone(),
                                #rsx_text.to_string(),
                            );
                        }
                        LazyNodes::new(move |__cx|{
                            if let Some(__text) = {
                                let read = __rsx_text_index.read();
                                // clone prevents deadlock on nested rsx calls
                                read.get(&__line_num).cloned()
                            } {
                                interpert_rsx(
                                    __cx,
                                    &__text,
                                    #captured
                                )
                            }
                            else {
                                panic!("rsx: line number {:?} not found in rsx index", __line_num);
                            }
                        })
                    }
                }
                .into()
            }
            #[cfg(not(feature = "hot_reload"))]
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
