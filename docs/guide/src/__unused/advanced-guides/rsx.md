# VNodes with RSX, HTML, and NodeFactory

Many modern frameworks provide a domain-specific-language for declaring user-interfaces. In the case of React, this language extension is called JSX and must be handled through additional dependencies and pre/post processors to transform your source code. With Rust, we can simply provide a procedural macro in the Dioxus dependency itself that mimics the JSX language.

With Dioxus, we actually ship two different macros - a macro that mimics JSX (the `html!` macro) and a macro that mimics Rust's native nested-struct syntax (the `rsx!` macro). These macros simply transform their inputs into NodeFactory calls.

For instance, this html! call:
```rust
html!(<div> "hello world" </div>)
```
becomes this NodeFactory call:
```rust
|f| f.element(
    dioxus_elements::div, // tag
    [], // listeners
    [], // attributes
    [f.static_text("hello world")], // children
    None // key
)
```
The NodeFactory API is fairly ergonomic, making it a viable option to use directly. The NodeFactory API is also compile-time correct and has incredible syntax highlighting support. We use what Rust calls a "unit type" - the `dioxus_elements::div` and associated methods to ensure that a `div` can only have attributes associated with `div`s. This lets us tack on relevant documentation, autocomplete support, and jump-to-definition for methods and attributes.

![Compile time correct syntax](../images/compiletimecorrect.png)

## html! macro

The html! macro supports a limited subset of the html standard. Rust's macro parsing tools are somewhat limited, so all text between tags _must be quoted_.

However, writing HTML by hand is a bit tedious - IDE tools for Rust don't support linting/autocomplete/syntax highlighting. We suggest using RSX - it's more natural for Rust programs and _does_ integrate well with Rust IDE tools.

```rust
let name = "jane";
let pending = false;
let count = 10;

dioxus::ssr::render_lazy(html! {
    <div>
        <p> "Hello, {name}!" </p>
        <p> "Status: {pending}!" </p>
        <p> "Count {count}!" </p>
    </div>
});
```

## rsx! macro

The rsx! macro is a VNode builder macro designed especially for Rust programs. Writing these should feel very natural, much like assembling a struct. VSCode also supports these with code folding, bracket-tabbing, bracket highlighting, section selecting, inline documentation, GOTO definition, and refactoring support.

When helpful, the Dioxus VSCode extension provides a way of converting a selection of HTML directly to RSX, so you can import templates from the web directly into your existing app.

It's also a bit easier on the eyes than HTML.

```rust
dioxus::ssr::render_lazy(rsx! {
    div {
        p {"Hello, {name}!"}
        p {"Status: {pending}!"}
        p {"Count {count}!"}
    }
});
```

In the next section, we'll cover the `rsx!` macro in more depth.
