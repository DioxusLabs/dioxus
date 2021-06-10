# VNode Macros

Dioxus comes preloaded with two macros for creating VNodes.

## html! macro

The html! macro supports a limited subset of the html standard. This macro will happily accept a copy-paste from something like tailwind builder. However, writing HTML by hand is a bit tedious - IDE tools for Rust don't support linting/autocomplete/syntax highlighting. RSX is much more natural for Rust programs and _does_ integrate well with Rust IDE tools.

There is also limited support for dynamic handlers, but it will function similarly to JSX.

You'll want to write RSX where you can, and in a future release we'll have a tool that automatically converts HTML to RSX.

```rust
#[fc]
fn Example(ctx: Context, name: &str, pending: bool, count: i32 ) -> VNode {
    ctx.render(html! {
        <div>
            <p> "Hello, {name}!" </p>
            <p> "Status: {pending}!" </p>
            <p> "Count {count}!" </p>
        </div>
    })
}
```

## rsx! macro

The rsx! macro is a VNode builder macro designed especially for Rust programs. Writing these should feel very natural, much like assembling a struct. VSCode also supports these with code folding, bracket-tabbing, bracket highlighting, and section selecting.

The Dioxus VSCode extension will eventually provide a macro to convert a selection of html! template and turn it into rsx!, so you'll never need to transcribe templates by hand.

It's also a bit easier on the eyes 🙂.

```rust
#[fc]
fn Example(ctx: Context, name: &str, pending: bool, count: i32 ) -> VNode {
    ctx.render(rsx! {
        div {
            p {"Hello, {name}!"}
            p {"Status: {pending}!"}
            p {"Count {count}!"}
        }
    })
}

```

Each element takes a comma-separated list of expressions to build the node. Roughly, here's how they work:

- `name: value` sets the property on this element.
- `"text"` adds a new text element
- `tag {}` adds a new child element
- `CustomTag {}` adds a new child component
- `{expr}` pastes the `expr` tokens literally. They must be IntoCtx<Vnode> to work properly

Commas are entirely optional, but might be useful to delineate between elements and attributes.

The `render` function provides an **extremely efficient** allocator for VNodes and text, so try not to use the `format!` macro in your components. Rust's default `ToString` methods pass through the global allocator, but all text in components is allocated inside a manually-managed Bump arena.

```rust
static Example: FC<()> = |ctx| {

    let text = "example";

    let g = async {
        wait(10).await;
        "hello"
    };

    let user_name = use_read_async(ctx, USERNAME);
    let title = ctx.suspend(user_name, |user_name| rsx!{ h1 { "Welcome back, {user_name}" } });


    ctx.render(rsx!{
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

                // bool-based classnames
                classes: [("big", true), ("small", false)]

                // Bool-based props
                // *must* be in the tuple form, cannot enter as a variable
                tag: ("type", false)

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
                    1 => rsx!(in ctx, h1 { "big" })
                    2 => rsx!(in ctx, h2 { "medium" })
                    _ => rsx!(in ctx, h3 { "small" })
                }}

                // Optionals
                {true.and_then(|f| rsx!{ h1 {"Conditional Rendering"} })}

                // Bool options
                {(rsx!{ h1 {"Conditional Rendering"}, true)}

                // Child nodes
                // Returns &[VNode]
                {ctx.children()}

                // Duplicating nodes
                // Clones the nodes by reference, so they are literally identical
                {{
                    let node = rsx!(in ctx, h1{ "TopNode" });
                    (0..10).map(|_| node.clone())
                }}

                // Any expression that is `IntoVNode`
                {expr}
            }
        }
    })
}
```
