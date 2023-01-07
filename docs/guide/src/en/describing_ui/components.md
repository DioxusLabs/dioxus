# Components

Just like you wouldn't want to write a complex program in a single, long, `main` function, you shouldn't build a complex UI in a single `App` function. Instead, you should break down the functionality of an app in logical parts called components.

A component is a Rust function, named in UpperCammelCase, that takes a `Scope` parameter and returns an `Element` describing the UI it wants to render. In fact, our `App` function is a component!

```rust
{{#include ../../../examples/hello_world_desktop.rs:component}}
```

> You'll probably want to add `#![allow(non_snake_case)]` to the top of your crate to avoid warnings about UpperCammelCase component names

A Component is responsible for some rendering task – typically, rendering an isolated part of the user interface. For example, you could have an `About` component that renders a short description of Dioxus Labs:

```rust
{{#include ../../../examples/components.rs:About}}
```

Then, you can render your component in another component, similarly to how elements are rendered:

```rust
{{#include ../../../examples/components.rs:App}}
```

![Screenshot containing the About component twice](./images/screenshot_about_component.png)

> At this point, it might seem like components are nothing more than functions. However, as you learn more about the features of Dioxus, you'll see that they are actually more powerful!
