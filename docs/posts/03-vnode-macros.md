# VNode Macros

Dioxus comes preloaded with two macros for creating VNodes.

## html! macro

The html! macro supports the html standard. This macro will happily accept a copy-paste from something like tailwind builder. Writing this one by hand is a bit tedious and doesn't come with much help from Rust IDE tools.

There is also limited support for dynamic handlers, but it will function similarly to JSX.

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

The rsx! macro is a VNode builder macro designed especially for Rust. Writing these should feel very natural, much like assembling a struct. VSCode also supports these with code folding, bracket-tabbing, bracket highlighting, and section selecting.

The Dioxus VSCode extension provides a function to convert a selection of html! template and turn it into rsx!, so you'll never need to transcribe templates by hand.

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

Lists must include commas, much like how struct definitions work.

```rust
static Example: FC<()> = |ctx| {

    ctx.render(rsx!{
        div {
            h1 { "Example" },
            p {
                // Props
                tag: "type",
                abc: 123,
                enabled: true,
                class: "big small wide short",

                // Children
                a { "abcder" },

                // Children with props
                h2 { "whatsup", class: "abc-123" },

                // Child components
                CustomComponent { a: 123, b: 456, key: "1" },

                // Iterators
                { 0..3.map(|i| rsx!{ h1 {"{:i}"} }) },

                // More rsx!, or even html!
                { rsx! { div { } } },
                { html! { <div> </div> } },

                // Any expression that is Into<VNode>
                {expr}
            }
        }
    })
}
```
