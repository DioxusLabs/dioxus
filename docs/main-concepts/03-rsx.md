# VNode Macros

Dioxus comes preloaded with two macros for creating VNodes.

## html! macro

The html! macro supports a limited subset of the html standard. This macro will happily accept a copy-paste from something like tailwind builder. However, writing HTML by hand is a bit tedious - IDE tools for Rust don't support linting/autocomplete/syntax highlighting. RSX is much more natural for Rust programs and _does_ integrate well with Rust IDE tools.

There is also limited support for dynamic handlers, but it will function similarly to JSX.

You'll want to write RSX where you can, and in a future release we'll have a tool that automatically converts HTML to RSX.

```rust
#[derive(PartialEq, Props)]
struct ExampleProps { name: &str, pending: bool, count: i32 }

fn Example(cx: Context<ExampleProps> ) -> VNode {
    let ExampleProps { name, pending, count } = cx.props;
    cx.render(html! {
        <div>
            <p> "Hello, {name}!" </p>
            <p> "Status: {pending}!" </p>
            <p> "Count {count}!" </p>
        </div>
    })
}
```

## rsx! macro

The rsx! macro is a VNode builder macro designed especially for Rust programs. Writing these should feel very natural, much like assembling a struct. VSCode also supports these with code folding, bracket-tabbing, bracket highlighting, section selecting, inline documentation, and GOTO definition (no rename support yet 😔 ).

The Dioxus VSCode extension will eventually provide a macro to convert a selection of html! template and turn it into rsx!, so you'll never need to transcribe templates by hand.

It's also a bit easier on the eyes 🙂 than HTML.

```rust
fn Example(cx: Context<ExampleProps>) -> VNode {
    cx.render(rsx! {
        div {
            // cx derefs to props so you can access fields directly
            p {"Hello, {cx.name}!"}
            p {"Status: {cx.pending}!"}
            p {"Count {cx.count}!"}
        }
    })
}
```

Each element takes a comma-separated list of expressions to build the node. Roughly, here's how they work:

- `name: value` sets a property on this element.
- `"text"` adds a new text element
- `tag {}` adds a new child element
- `CustomTag {}` adds a new child component
- `{expr}` pastes the `expr` tokens literally. They must be `IntoIterator<T> where T: IntoVnode` to work properly

Commas are entirely optional, but might be useful to delineate between elements and attributes.

The `render` function provides an **extremely efficient** allocator for VNodes and text, so try not to use the `format!` macro in your components. Rust's default `ToString` methods pass through the global allocator, but all text in components is allocated inside a manually-managed Bump arena. To push you in the right direction, all text-based attributes take `std::fmt::Arguments` directly, so you'll want to reach for `format_args!` when the built-in `f-string` interpolation just doesn't cut it.

```rust
pub static Example: FC<()> = |cx| {

    let text = "example";

    cx.render(rsx!{
        div {
            h1 { "Example" },

            {title}

            // fstring interpolation
            "{text}"

            p {
                // Attributes
                tag: "type",

                // Anything that implements display can be an attribute
                abc: 123,
                enabled: true,

                // attributes also supports interpolation
                // `class` is not a restricted keyword unlike JS and ClassName
                class: "big small wide short {text}",

                class: format_args!("attributes take fmt::Arguments. {}", 99)

                tag: {"these tokens are placed directly"}

                // Children
                a { "abcder" },

                // Children with attributes
                h2 { "hello", class: "abc-123" },

                // Child components
                CustomComponent { a: 123, b: 456, key: "1" },

                // Child components with paths
                crate::components::CustomComponent { a: 123, b: 456, key: "1" },

                // Iterators
                { 0..3.map(|i| rsx!( h1 {"{:i}"} )) },

                // More rsx!, or even html!
                { rsx! { div { } } },
                { html! { <div> </div> } },

                // Matching
                // Requires rendering the nodes first.
                // rsx! is lazy, and the underlying closures cannot have the same type
                // Rendering produces the VNode type
                {match rand::gen_range::<i32>(1..3) {
                    1 => rsx!(in cx, h1 { "big" })
                    2 => rsx!(in cx, h2 { "medium" })
                    _ => rsx!(in cx, h3 { "small" })
                }}

                // Optionals
                {true.and_then(|f| rsx!{ h1 {"Conditional Rendering"} })}

                // Child nodes
                // Returns &[VNode]
                {cx.children()}

                // Duplicating nodes
                // Clones the nodes by reference, so they are literally identical
                {{
                    let node = rsx!(in cx, h1{ "TopNode" });
                    (0..10).map(|_| node.clone())
                }}

                // Any expression that is `IntoVNode`
                {expr}
            }
        }
    })
}
```
