use proc_macro::TokenStream;
use quote::ToTokens;
use rsx::{AS_HTML, AS_RSX};
use syn::parse_macro_input;

pub(crate) mod htm;
pub(crate) mod ifmt;
pub(crate) mod props;
pub(crate) mod rsx;

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
/// const Example: FC<()> = |(cx, props)|{
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
///                 true => rsx!(cx, h1 {"Top text"}),
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
///                 rsx!(cx, h1 {"Top text"})
///             } else {
///                 rsx!(cx, h1 {"Bottom text"})
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
///     pub fn Baller(cx: Context<()>) -> DomTree {
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
/// pub fn Taller(cx: Context<TallerProps>) -> DomTree {
///     let b = true;
///     todo!()
/// }
/// ```
#[proc_macro]
pub fn rsx(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody<AS_RSX>>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}

/// The html! macro makes it easy for developers to write jsx-style markup in their components.
///
/// ## Complete Reference Guide:
/// ```
/// const Example: FC<()> = |(cx, props)|{
///     let formatting = "formatting!";
///     let formatting_tuple = ("a", "b");
///     let lazy_fmt = format_args!("lazily formatted text");
///     cx.render(html! {
///         <div>
///             <div />
///             <div> </div>
///             <h1>"Some text"</h1>
///             <h1>"Some text with {formatting}"</h1>
///             <h1>"Formatting basic expressions {formatting_tuple.0} and {formatting_tuple.1}"</h1>
///             <h2>
///                 "Multiple"
///                 "Text"
///                 "Blocks"
///                 "Use comments as separators in html"
///             </h2>
///             <div>
///                 <h1>"multiple"</h1>
///                 <h2>"nested"</h2>
///                 <h3>"elements"</h3>
///             </div>
///             <div class="my special div">
///                 <h1>"Headers and attributes!"</h1>
///             </div>
///             <div class: lazy_fmt, id=format_args!("attributes can be passed lazily with std::fmt::Arguments")>
///                 <div class: {
///                     const WORD: &str = "expressions";
///                     format_args!("Arguments can be passed in through curly braces for complex {}", WORD)
///                 } />
///             </div>
///             {rsx!(p { "More templating!" })}
///             {html!(<p>"Even HTML templating!!"</p>)}
///             {(0..10).map(|i| html!(<li>"{i}"</li>))}
///             {{
///                 let data = std::collections::HashMap::<&'static str, &'static str>::new();
///                 // Iterators *should* have keys when you can provide them.
///                 // Keys make your app run faster. Make sure your keys are stable, unique, and predictable.
///                 // Using an "ID" associated with your data is a good idea.
///                 data.into_iter().map(|(k, v)| rsx!(<li key="{k}"> "{v}" </li>))
///             }}

///             // Matching
///             // Matching will throw a Rust error about "no two closures are the same type"
///             // To fix this, call "render" method or use the "in" syntax to produce VNodes.
///             // There's nothing we can do about it, sorry :/ (unless you want *really* unhygenic macros)
///             {match true {
///                 true => rsx!(cx, <h1>"Top text"</h1>),
///                 false => rsx!(cx, <h1>"Bottom text"</h1>),
///             }}
///
///             // Conditional rendering
///             // Dioxus conditional rendering is based around None/Some. We have no special syntax for conditionals.
///             // You can convert a bool condition to rsx! with .then and .or
///             {true.then(|| html!(<div />))}
///
///             // True conditions need to be rendered (same reasons as matching)
///             {if true {
///                 html!(cx, <h1>"Top text"</h1>)
///             } else {
///                 html!(cx, <h1>"Bottom text"</h1>)
///             }}
///
///             // returning "None" is a bit noisy... but rare in practice
///             {None as Option<()>}
///
///             // Use the Dioxus type-alias for less noise
///             {NONE_ELEMENT}
///
///             // can also just use empty fragments
///             <Fragment />
///
///             // Fragments let you insert groups of nodes without a parent.
///             // This lets you make components that insert elements as siblings without a container.
///             <div> "A" </div>
///             <Fragment>
///                 <div> "B" </div>
///                 <div> "C" </div>
///                 <Fragment>
///                     "D"
///                     <Fragment>
///                         "heavily nested fragments is an antipattern"
///                         "they cause Dioxus to do unnecessary work"
///                         "don't use them carelessly if you can help it"
///                     </Fragment>
///                 </Fragment
///             </Fragment>
///
///             // Components
///             // Can accept any paths
///             // Notice how you still get syntax highlighting and IDE support :)
///             <Baller />
///             <baller::Baller />
///             <crate::baller::Baller />
///
///             // Can take properties
///             <Taller a="asd" />
///
///             // Can take optional properties
///             <Taller a="asd" />
///
///             // Can pass in props directly as an expression
///             {{
///                 let props = TallerProps {a: "hello"};
///                 html!(<Taller ..{props} />)
///             }}
///
///             // Spreading can also be overridden manually
///             <Taller {..TallerProps { a: "ballin!" }} a="not ballin!" />
///
///             // Can take children too!
///             <Taller a="asd">
///                 <div> "hello world!" </div>
///             </Taller>
///         }
///     })
/// };
///
/// mod baller {
///     use super::*;
///     pub struct BallerProps {}
///
///     /// This component totally balls
///     pub fn Baller(cx: Context<()>) -> DomTree {
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
/// pub fn Taller(cx: Context<TallerProps>) -> DomTree {
///     let b = true;
///     todo!()
/// }
/// ```
#[proc_macro]
pub fn html(s: TokenStream) -> TokenStream {
    match syn::parse::<rsx::CallBody<AS_HTML>>(s) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
