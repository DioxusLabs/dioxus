<div>
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>A concurrent, functional, virtual DOM for Rust</strong>
  </p>
</div>

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

All Dioxus apps are built by composing functions that start with a capital letter and return an `Element`.

To launch an app, we use the `launch` method and use features in `Cargo.toml` to specify which renderer we want to use. In the launch function, we pass the app's root `Component`.

```rust, no_run
use dioxus::prelude::*;

fn main() {
    launch(App);
}

// The #[component] attribute streamlines component creation.
// It's not required, but highly recommended. It will lint incorrect component definitions and help you create props structs.
#[component]
fn App() -> Element {
    rsx! { "hello world!" }
}
```

## Elements & your first component

You can use the `rsx!` macro to create elements with a jsx-like syntax.
Any element in `rsx!` can have attributes, listeners, and children. For
consistency, we force all attributes and listeners to be listed _before_
children.

```rust, no_run
# use dioxus::prelude::*;
let value = "123";

rsx! {
    div {
        class: "my-class {value}",                  // <--- attribute
        onclick: move |_| println!("clicked!"),   // <--- listener
        h1 { "hello world" }                       // <--- child
    }
};
```

The `rsx!` macro accepts attributes in "struct form". Any rust expression contained within curly braces that implements [`IntoDynNode`](dioxus_core::IntoDynNode) will be parsed as a child. We make two exceptions: both `for` loops and `if` statements are parsed where their body is parsed as a rsx nodes.

```rust, no_run
# use dioxus::prelude::*;
rsx! {
    div {
        for _ in 0..10 {
            span { "hello world" }
        }
    }
};
```

Putting everything together, we can write a simple component that renders a list of
elements:

```rust, no_run
# use dioxus::prelude::*;
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
component we design must take some Properties. For components with no explicit properties, we can omit the type altogether.

In Dioxus, all properties are memorized by default with Clone and PartialEq. For props you can't clone, simply wrap the fields in a [`ReadOnlySignal`](dioxus_signals::ReadOnlySignal) and Dioxus will handle converting types for you.

```rust, no_run
# use dioxus::prelude::*;
# #[component] fn Header(title: String, color: String) -> Element { todo!() }
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

The `#[component]` macro will help us automatically create a props struct for our component:

```rust, no_run
# use dioxus::prelude::*;
// The component macro turns the arguments for our function into named fields we can pass in to the component in rsx
#[component]
fn Header(title: String, color: String) -> Element {
    rsx! {
        div {
            background_color: "{color}",
            h1 { "{title}" }
        }
    }
}
```

> You can read more about props in the [reference](https://dioxuslabs.com/learn/0.5/reference/component_props).

## Hooks

While components are reusable forms of UI elements, hooks are reusable forms
of logic. Hooks provide a way of retrieving state from Dioxus' internal `Scope` and using
it to render UI elements.

By convention, all hooks are functions that should start with `use_`. We can
use hooks to define the state and modify it from within listeners.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn App() -> Element {
    // The use signal hook runs once when the component is created and then returns the current value every run after the first
    let name = use_signal(|| "world");

    rsx! { "hello {name}!" }
}
```

Hooks are sensitive to how they are used. To use hooks, you must abide by the
["rules of hooks"](https://dioxuslabs.com/learn/0.5/reference/hooks#rules-of-hooks):

- Hooks can only be called in the body of a component or another hook. Not inside of another expression like a loop, conditional or function call.
- Hooks should start with "use\_"

In a sense, hooks let us add a field of state to our component without declaring
an explicit state struct. However, this means we need to "load" the struct in the right
order. If that order is wrong, then the hook will pick the wrong state and panic.

Dioxus includes many built-in hooks that you can use in your components. If those hooks don't fit your use case, you can also extend Dioxus with [custom hooks](https://dioxuslabs.com/learn/0.5/cookbook/state/custom_hooks).

## Putting it all together

Using components, rsx, and hooks, we can build a simple app.

```rust, no_run
use dioxus::prelude::*;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div { "Count: {count}" }
        button { onclick: move |_| count += 1, "Increment" }
        button { onclick: move |_| count -= 1, "Decrement" }
    }
}
```

## Conclusion

This overview doesn't cover everything. Make sure to check out the [tutorial](https://dioxuslabs.com/learn/0.5/guide) and [reference](https://dioxuslabs.com/learn/0.5/reference) on the official
website for more details.

Beyond this overview, Dioxus supports:

- [Server-side rendering](https://dioxuslabs.com/learn/0.5/reference/fullstack)
- Concurrent rendering (with async support)
- Web/Desktop/Mobile support
- Pre-rendering and hydration
- Fragments, and Suspense
- Inline-styles
- [Custom event handlers](https://dioxuslabs.com/learn/0.5/reference/event_handlers#handler-props)
- Custom elements
- Basic fine-grained reactivity (IE SolidJS/Svelte)
- and more!

Build cool things ✌️
