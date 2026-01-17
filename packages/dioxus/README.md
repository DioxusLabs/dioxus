<div align="center">
    <img
        src="https://github.com/user-attachments/assets/6c7e227e-44ff-4e53-824a-67949051149c"
        alt="Build web, desktop, and mobile apps with a single codebase."
        width="100%"
        class="darkmode-image"
    >
    <div>
        <a href=https://dioxuslabs.com/learn/0.7/getting_started>Getting Started</a> | <a href="https://dioxuslabs.com/learn/0.7/">Book (0.7)</a> | <a href="https://github.com/DioxusLabs/dioxus/tree/main/examples">Examples</a>
    </div>
</div>

---

Dioxus is a framework for building cross-platform apps in Rust. With one codebase, you can build web, desktop, and mobile apps with fullstack server functions. Dioxus is designed to be easy to learn for developers familiar with web technologies like HTML, CSS, and JavaScript.

<div align="center">
    <img src="https://github.com/user-attachments/assets/dddae6a9-c13b-4a88-84e8-dc98c1286d2a" alt="App with dioxus" height="600px">
</div>

## At a glance

Dioxus is crossplatform app framework that empowers developer to build beautiful, fast, type-safe apps with Rust. By default, Dioxus apps are declared with HTML and CSS. Dioxus includes a number of useful features:

- Hotreloading of RSX markup and assets
- Interactive CLI with logging, project templates, linting, and more
- Integrated bundler for deploying to the web, macOS, Linux, and Windows
- Support for modern web features like SSR, Hydration, and HTML streaming
- Direct access to system APIs through JNI (Android), CoreFoundation (Apple), and web-sys (web)
- Type-safe application routing and server functions

## Quick start

To get started with Dioxus, you'll want to grab the dioxus-cli tool: `dx`. We distribute `dx` with `cargo-binstall` - if you already have binstall skip this step.

```shell
# skip if you already have cargo-binstall
cargo install cargo-binstall

# install the precompiled `dx` tool
cargo binstall dioxus-cli

# create a new app, following the template
dx new my-app && cd my-app

# and then serve!
dx serve --desktop
```

## Your first app

All Dioxus apps are built by composing functions return an `Element`.

To launch an app, we use the `launch` method. In the launch function, we pass the app's root `Component`.

```rust, no_run
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
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

We can compose these function components to build a complex app. Each new component takes some Properties. For components with no explicit properties, we can omit the type altogether.

In Dioxus, all properties are memoized by default with `Clone` and `PartialEq`. For props you can't clone, simply wrap the fields in a [`ReadSignal`](dioxus_signals::ReadSignal) and Dioxus will handle converting types for you.

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

> You can read more about props in the [reference](https://dioxuslabs.com/learn/0.7/essentials/ui/components).

## Hooks

While components are reusable forms of UI elements, hooks are reusable forms of logic. Hooks provide a way of retrieving state from Dioxus' internal `Scope` and using
it to render UI elements.

By convention, all hooks are functions that should start with `use_`. We can use hooks to define the state and modify it from within listeners.

```rust, no_run
# use dioxus::prelude::*;
#[component]
fn App() -> Element {
    // The use signal hook runs once when the component is created and then returns the current value every run after the first
    let name = use_signal(|| "world");

    rsx! { "hello {name}!" }
}
```

Hooks are sensitive to how they are used. To use hooks, you must abide by the ["rules of hooks"](https://dioxuslabs.com/learn/0.7/essentials/basics/hooks#rules-of-hooks):

- Hooks can only be called in the body of a component or another hook. Not inside of another expression like a loop, conditional or function call.
- Hooks should start with "use\_"

Hooks let us add a field of state to our component without declaring an explicit state struct. However, this means we need to "load" the struct in the right order. If that order is wrong, then the hook will pick the wrong state and panic.

Dioxus includes many built-in hooks that you can use in your components. If those hooks don't fit your use case, you can also extend Dioxus with custom hooks.

## Putting it all together

Using components, rsx, and hooks, we can build a simple app.

```rust, no_run
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
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

This overview doesn't cover everything. Make sure to check out the [tutorial](https://dioxuslabs.com/learn/0.7/tutorial) and [guides](https://dioxuslabs.com/learn/0.7/tutorial) on the official
website for more details.

Beyond this overview, Dioxus supports:

- Server-side rendering
- Concurrent rendering (with async support)
- Web/Desktop/Mobile support
- Pre-rendering and hydration
- Fragments, and Suspense
- Inline-styles
- Custom event handlers
- Custom elements
- Basic fine-grained reactivity (IE SolidJS/Svelte)
- and more!

Build cool things ✌️
