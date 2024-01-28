# dioxus-core

dioxus-core is a fast and featureful VirtualDom implementation written in and for Rust.

# Features

- Functions as components
- Hooks for local state
- Task pool for spawning futures
- Template-based architecture
- Asynchronous components
- Suspense boundaries
- Error boundaries through the `anyhow` crate
- Customizable memoization

If you are just starting, check out the Guides first.

# General Theory

The dioxus-core `VirtualDom` object is built around the concept of a `Template`. Templates describe a layout tree known at compile time with dynamic parts filled at runtime.

Each component in the VirtualDom works as a dedicated render loop where re-renders are triggered by events external to the VirtualDom, or from the components themselves.

When each component re-renders, it must return an `Element`. In Dioxus, the `Element` type is an alias for `Result<VNode>`. Between two renders, Dioxus compares the inner `VNode` object and calculates the differences between the dynamic portions of each internal `Template`. If any attributes or elements are different between the old layout and the new layout, Dioxus will write modifications to the `Mutations` object.

Dioxus expects the target renderer to save its nodes in a list. Each element is given a numerical ID which can be used to directly index into that list for O(1) lookups.

# Usage

All Dioxus apps start as just a function that takes the [`Scope`] object and returns an [`Element`].

The `dioxus` crate exports the `rsx` macro which transforms a helpful, simpler syntax of Rust into the logic required to build Templates.

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


We can then wait for any asynchronous components or pending futures using the `wait_for_work()` method. If we have a deadline, then we can use render_with_deadline instead:
```rust
# #![allow(unused)]
# use dioxus::prelude::*;

# use std::time::Duration;
# async fn wait(mut dom: VirtualDom) {
// Wait for the dom to be marked dirty internally
dom.wait_for_work().await;
# }
```

If an event occurs from outside the VirtualDom while waiting for work, then we can cancel the wait using a `select!` block and inject the event.

```rust, ignore
loop {
    select! {
        evt = real_dom.event() => dom.handle_event("click", evt.data, evt.element, evt.bubbles),
        _ = dom.wait_for_work() => {}
    }

    // Render any work without blocking the main thread for too long
    let mutations = dom.render_with_deadline(tokio::time::sleep(Duration::from_millis(10)));

    // And then apply the edits
    real_dom.apply(mutations);
}

```

## Internals

Dioxus-core builds off the many frameworks that came before it. Notably, Dioxus borrows these concepts:

- React: hooks, concurrency, suspense
- Dodrio: bump allocation, double buffering, and some diffing architecture

Dioxus-core hits a very high level of parity with mature frameworks. However, Dioxus also brings some new unique features:

- managed lifetimes for borrowed data
- placeholder approach for suspended vnodes
- fiber/interruptible diffing algorithm
- custom memory allocator for VNodes and all text content
- support for fragments w/ lazy normalization
- slab allocator for scopes
- mirrored-slab approach for remote VirtualDoms
- dedicated subtrees for rendering into separate contexts from the same app

There's certainly more to the story, but these optimizations make Dioxus memory use and allocation count extremely minimal. For an average application, no allocations may be needed once the app has been loaded. Only when new components are added to the dom will allocations occur. For a given component, the space of old VNodes is dynamically recycled as new nodes are added. Additionally, Dioxus tracks the average memory footprint of previous components to estimate how much memory allocate for future components.

All in all, Dioxus treats memory as a valuable resource. Combined with the memory-efficient footprint of Wasm compilation, Dioxus apps can scale to thousands of components and still stay snappy.

## Goals

The final implementation of Dioxus must:

- Be **fast**. Allocators are typically slow in Wasm/Rust, so we should have a smart way of allocating.
- Be memory efficient. Servers should handle tens of thousands of simultaneous VDoms with no problem.
- Be concurrent. Components should be able to pause rendering to let the screen paint the next frame.
- Be disconnected from a specific renderer (no WebSys dependency in the core crate).
- Support server-side-rendering (SSR). VNodes should render to a string that can be served via a web server.
- Be "live". Components should be able to be both server-rendered and client rendered without needing frontend APIs.
- Be modular. Components and hooks should work anywhere without worrying about the target platform.
