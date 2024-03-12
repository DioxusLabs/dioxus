# dioxus-core

`dioxus-core` provides a fast and featureful VirtualDom implementation for Rust.

```rust, ignore
use dioxus_core::prelude::*;

let vdom = VirtualDom::new(app);
let real_dom = SomeRenderer::new();

loop {
    select! {
        evt = real_dom.event() => vdom.handle_event(evt),
        _ = vdom.wait_for_work() => {}
    }
    vdom.render(&mut real_dom)
}

# fn app() -> Element { None }
# struct SomeRenderer; impl SomeRenderer { fn new() -> SomeRenderer { SomeRenderer; } async fn event() -> () { unimplemented!() } }
```

## Features

A virtualdom is an efficient and flexible tree datastructure that allows you to manage state for a graphical user interface. The Dioxus VirtualDom is perhaps the most fully-featured virtualdom implementation in Rust and powers renderers running across Web, Desktop, Mobile, SSR, TUI, LiveView, and more. When you use the Dioxus VirtualDom, you immediately enable users of your renderer to leverage the wide ecosystem of Dioxus components, hooks, and associated tooling.

Some features of `dioxus-core` include:

- UI components are just functions
- State is provided by hooks
- Deep integration with async
- Strong focus on performance
- Integrated hotreloading support
- Extensible system for UI elements and their attributes

If you are just starting, check out the Guides first.

## Understanding the implementation

`dioxus-core` is designed to be a lightweight crate that. It exposes a number of flexible primitives without being deeply concerned about the intracices of state management itself. We proivde a number of useful abstractions built on these primitives in the `dioxus-hooks` crate as well as the `dioxus-signals` crate.

The important abstractions to understand are:
- The [`VirtualDom`]
- The [`Component`] and its [`Properties`]
- Handling events
- Working with async
- Suspense

## Usage

The `dioxus` crate exports the `rsx` macro which transforms a helpful, simpler syntax of Rust.

First, start with your app:

```rust
# use dioxus::dioxus_core::Mutations;
use dioxus::prelude::*;

// First, declare a root component
fn app() -> Element {
    rsx!{
        div { "hello world" }
    }
}

fn main() {
    // Next, create a new VirtualDom using this app as the root component.
    let mut dom = VirtualDom::new(app);

    // The initial render of the dom will generate a stream of edits for the real dom to apply
    let mutations = dom.rebuild_to_vec();
}
```

## Contributing
- Check out the website [section on contributing](https://dioxuslabs.com/learn/0.4/contributing).
- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- [Join](https://discord.gg/XgGxMSkvUM) the discord and ask questions!


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>


## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
