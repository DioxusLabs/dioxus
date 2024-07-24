<div>
  <h1>🌗🚀 Dioxus (lib)</h1>
  <p>
    <strong>A concurrent, functional, virtual DOM for Rust</strong>
  </p>
</div>

> This crate implements dioxus-lib which is a renderer-free version of Dioxus. This crate is intended to be used by library authors who need a stable core version of dioxus withuot pulling in renderer-related dependencies on accident.

# Resources

This overview provides a brief introduction to Dioxus. For a more in-depth guide, make sure to check out:

- [Getting Started](https://dioxuslabs.com/learn/0.5/getting_started)
- [Book (0.5)](https://dioxuslabs.com/learn/0.5)
- [Examples](https://github.com/DioxusLabs/example-projects)

# Overview and Goals

Dioxus makes it easy to quickly build complex user interfaces with Rust. Any Dioxus app can run in the web browser,
as a desktop app, as a mobile app, or anywhere else provided you build the right renderer.

Dioxus is heavily inspired by React, supporting many of the same concepts:

- Hooks for state
- VirtualDom & diffing
- Concurrency, fibers, and asynchronous rendering
- JSX-like templating syntax

If you know React, then you know Dioxus.

Dioxus is _substantially_ more performant than many of the other Rust UI libraries (Yew/Percy) and is _significantly_ more performant
than React—roughly competitive with InfernoJS.

Remember: Dioxus is a library for declaring interactive user interfaces—it is not a dedicated renderer. Most 1st party renderers for Dioxus currently only support web technologies.

## Brief Overview

All Dioxus apps are built by composing functions that return an `Element`.

To launch an app, we use the `launch` method and use features in `Cargo.toml` to specify which renderer we want to use. In the launch function, we pass the app's root `Component`.

```rust, no_run
use dioxus::prelude::*;

fn main() {
    launch(App);
}

// The #[component] attribute streamlines component creation.
// It's not required, but highly recommended. For example, UpperCamelCase components will not generate a warning.
#[component]
fn App() -> Element {
    rsx! { "hello world!" }
}
```

## Elements & your first component

To assemble UI trees with Dioxus, you need to use the `render` function on
something called `LazyNodes`. To produce `LazyNodes`, you can use the `rsx!`
macro or the NodeFactory API. For the most part, you want to use the `rsx!`
macro.

Any element in `rsx!` can have attributes, listeners, and children. For
consistency, we force all attributes and listeners to be listed _before_
children.

```rust, ignore
let value = "123";

rsx! {
    div {
        class: "my-class {value}",                  // <--- attribute
        onclick: move |_| info!("clicked!"),   // <--- listener
        h1 { "hello world" },                       // <--- child
    }
}
```

The `rsx!` macro accepts attributes in "struct form". Any rust expression contained within curly braces that implements `IntoIterator<Item = impl IntoVNode>` will be parsed as a child. We make two exceptions: both `for` loops and `if` statements are parsed where their body is parsed as a child.

```rust, ignore
rsx! {
    div {
        for _ in 0..10 {
            span { "hello world" }
        }
    }
}
```

The `rsx!` macro is what generates the `Element` that our components return.

```rust, ignore
#[component]
fn Example() -> Element {
    rsx!{ "hello world" }
}
```

Putting everything together, we can write a simple component that renders a list of
elements:

```rust, ignore
#[component]
fn App() -> Element {
    let name = "dave";
    rsx! {
        h1 { "Hello, {name}!" }
        div { class: "my-class", id: "my-id",
            for i in 0..5 {
                div { "FizzBuzz: {i}" }
            }
        }
    }
}
```

## Components

We can compose these function components to build a complex app. Each new
component we design must take some Properties. For components with no explicit
properties we can omit the type altogether.

In Dioxus, all properties are memoized by default, and this implement both Clone and PartialEq. For props you can't clone, simply wrap the fields in a ReadOnlySignal and Dioxus will handle the wrapping for you.

```rust, ignore
#[component]
fn App() -> Element {
    rsx! {
        Header {
            title: "My App",
            color: "red",
        }
    }
}
```

Our `Header` component takes a `title` and a `color` property, which we
declare on an explicit `HeaderProps` struct.

```rust, ignore
// The `Props` derive macro lets us add additional functionality to how props are interpreted.
#[derive(Props, PartialEq)]
struct HeaderProps {
    title: String,
    color: String,
}

#[component]
fn Header(props: HeaderProps) -> Element {
    rsx! {
        div {
            background_color: "{props.color}"
            h1 { "{props.title}" }
        }
    }
}
```

The `#[component]` macro also allows you to derive the props
struct from function arguments:

```rust, ignore
#[component]
fn Header(title: String, color: String) -> Element {
    rsx! {
        div {
            background_color: "{color}"
            h1 { "{title}" }
        }
    }
}
```

Components that begin with an uppercase letter may be called with
the traditional (for React) curly-brace syntax like so:

```rust, ignore
rsx! {
    Header { title: "My App" }
}
```

## Hooks

While components are reusable forms of UI elements, hooks are reusable forms
of logic. Hooks provide us a way of retrieving state from Dioxus' internal `Scope` and using
it to render UI elements.

By convention, all hooks are functions that should start with `use_`. We can
use hooks to define the state and modify it from within listeners.

```rust, ignore
#[component]
fn App() -> Element {
    let name = use_signal(|| "world");

    rsx! { "hello {name}!" }
}
```

Hooks are sensitive to how they are used. To use hooks, you must abide by the
["rules of hooks"](https://dioxuslabs.com/learn/0.5/reference/hooks#rules-of-hooks):

- Functions with "use\_" should not be called in callbacks
- Functions with "use\_" should not be called out of order
- Functions with "use\_" should not be called in loops or conditionals

In a sense, hooks let us add a field of state to our component without declaring
an explicit state struct. However, this means we need to "load" the struct in the right
order. If that order is wrong, then the hook will pick the wrong state and panic.

Most hooks you'll write are simply compositions of other hooks:

```rust, ignore
fn use_username(d: Uuid) -> bool {
    let users = use_context::<Users>();
    users.get(&id).map(|user| user.logged_in).ok_or(false)
}
```

To create entirely new foundational hooks, we can use the `use_hook` method.

```rust, ignore
fn use_mut_string() -> String {
    use_hook(|_| "Hello".to_string())
}
```

If you want to extend Dioxus with some new functionality, you'll probably want to implement a new hook from scratch.

## Putting it all together

Using components, templates, and hooks, we can build a simple app.

```rust, ignore
use dioxus::prelude::*;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);

    rsx!(
        div { "Count: {count}" }
        button { onclick: move |_| count += 1, "Increment" }
        button { onclick: move |_| count -= 1, "Decrement" }
    )
}
```

## Features

This overview doesn't cover everything. Make sure to check out the tutorial and reference guide on the official
website for more details.

Beyond this overview, Dioxus supports:

- Server-side rendering
- Concurrent rendering (with async support)
- Web/Desktop/Mobile support
- Pre-rendering and rehydration
- Fragments, Portals, and Suspense
- Inline-styles
- Custom event handlers
- Custom elements
- Basic fine-grained reactivity (IE SolidJS/Svelte)
- and more!

Build cool things ✌️
