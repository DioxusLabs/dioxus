# dioxus-core

`dioxus-core` provides a fast and featureful VirtualDom implementation for Rust.

```rust, no_run
# tokio::runtime::Runtime::new().unwrap().block_on(async {
use dioxus_core::{
    VirtualDom, Event, Element, Mutations, VNode, ElementId, RenderTargetId,
};

let mut vdom = VirtualDom::new(app);
let mut real_dom = SomeRenderer::new();
vdom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
vdom.rebuild();
real_dom.flush(vdom.render_target_mut::<Mutations>(RenderTargetId::ROOT).unwrap());

loop {
    tokio::select! {
        evt = real_dom.event() => {
            let evt = Event::new(evt, true);
            vdom.runtime().handle_event("onclick", evt, ElementId(0))
        },
        _ = vdom.wait_for_work() => {}
    }
    vdom.render_concurrent().await;
    real_dom.flush(vdom.render_target_mut::<Mutations>(RenderTargetId::ROOT).unwrap());
}

# fn app() -> Element { VNode::empty() }
# struct SomeRenderer;
# impl SomeRenderer {
#     fn new() -> Self { Self }
#     async fn event(&self) -> std::rc::Rc<dyn std::any::Any> { unimplemented!() }
#     fn flush(&mut self, _: &Mutations) {}
# }
# });
```

## Features

A virtualdom is an efficient and flexible tree data structure that allows you to manage state for a graphical user interface. The Dioxus VirtualDom is perhaps the most fully-featured virtualdom implementation in Rust and powers renderers running across Web, Desktop, Mobile, SSR, TUI, LiveView, and more. When you use the Dioxus VirtualDom, you immediately enable users of your renderer to leverage the wide ecosystem of Dioxus components, hooks, and associated tooling.

Some features of `dioxus-core` include:

- UI components are just functions
- State is provided by hooks
- Deep integration with async
- Strong focus on performance
- Integrated hotreloading support
- Extensible system for UI elements and their attributes

If you are just starting, check out the Guides first.

## Understanding the implementation

`dioxus-core` is designed to be a lightweight crate that. It exposes a number of flexible primitives without being deeply concerned about the intracices of state management itself. We provide a number of useful abstractions built on these primitives in the `dioxus-hooks` crate as well as the `dioxus-signals` crate.

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
# use dioxus::dioxus_core::VirtualDom;
use dioxus::prelude::*;

// First, declare a root component
fn app() -> Element {
    rsx! {
        div { "hello world" }
    }
}

fn main() {
    // Next, create a new VirtualDom using this app as the root component.
    let mut dom = VirtualDom::new(app);

    // Register an in-memory collector at the root target. The initial render
    // populates it with mutations.
    dom.insert_render_target(
        dioxus_core::RenderTargetId::ROOT,
        dioxus_core::Mutations::default(),
    );
    dom.rebuild();
}
```

## Contributing

- Check out the website [section on contributing](https://dioxuslabs.com/learn/0.7/beyond/contributing).
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
