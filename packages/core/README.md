# Dioxus-core

This is the core crate for the Dioxus Virtual DOM. This README will focus on the technical design and layout of this Virtual DOM implementation. If you want to read more about using Dioxus, then check out the Dioxus crate, documentation, and website.

To build new apps with Dioxus or to extend the ecosystem with new hooks or components, use the higher-level `dioxus` crate with the appropriate feature flags.


```rust, ignore
fn app(cx: Scope) -> Element {
    rsx!(cx, div { "hello world" })
}

fn main() {
    let mut renderer = SomeRenderer::new();

    // Creating a new virtualdom from a component
    let mut dom = VirtualDom::new(app);

    // Patching the renderer with the changes to draw the screen
    let edits = dom.rebuild();
    renderer.apply(edits);

    // Injecting events
    dom.handle_message(SchedulerMsg::Event(UserEvent {
        scope_id: None,
        priority: EventPriority::High,
        element: ElementId(0),
        name: "onclick",
        data: Arc::new(()),
    }));

    // polling asynchronously
    dom.wait_for_work().await;

    // working with a deadline
    if let Some(edits) = dom.work_with_deadline(|| false) {
        renderer.apply(edits);
    }

    // getting state of scopes
    let scope = dom.get_scope(ScopeId(0)).unwrap();

    // iterating through the tree
    match scope.root_node() {
        VNodes::Text(vtext) => dbg!(vtext),
        VNodes::Element(vel) => dbg!(vel),
        _ => todo!()
    }
}
```

## Internals

Dioxus-core builds off the many frameworks that came before it. Notably, Dioxus borrows these concepts:

- React: hooks, concurrency, suspense
- Dodrio: bump allocation, double buffering, and some diffing architecture

Dioxus-core leverages some really cool techniques and hits a very high level of parity with mature frameworks. However, Dioxus also brings some new unique features:

- managed lifetimes for borrowed data
- placeholder approach for suspended vnodes
- fiber/interruptible diffing algorithm
- custom memory allocator for vnodes and all text content
- support for fragments w/ lazy normalization
- slab allocator for scopes
- mirrored-slab approach for remote vdoms
- dedicated subtrees for rendering into separate contexts from the same app

There's certainly more to the story, but these optimizations make Dioxus memory use and allocation count extremely minimal. For an average application, it is possible that zero allocations will need to be performed once the app has been loaded. Only when new components are added to the dom will allocations occur. For a given component, the space of old VNodes is dynamically recycled as new nodes are added. Additionally, Dioxus tracks the average memory footprint of previous components to estimate how much memory allocate for future components.

All in all, Dioxus treats memory as a valuable resource. Combined with the memory-efficient footprint of Wasm compilation, Dioxus apps can scale to thousands of components and still stay snappy.

## Goals

The final implementation of Dioxus must:

- Be **fast**. Allocators are typically slow in Wasm/Rust, so we should have a smart way of allocating.
- Be memory efficient. Servers should handle tens of thousands of simultaneous VDoms with no problem.
- Be concurrent. Components should be able to pause rendering to let the screen paint the next frame.
- Be disconnected from a specific renderer (no WebSys dependency in the core crate).
- Support server-side-rendering (SSR). VNodes should render to a string that can be served via a web server.
- Be "live". Components should be able to be both server rendered and client rendered without needing frontend APIs.
- Be modular. Components and hooks should be work anywhere without worrying about target platform.


## Safety

Dioxus uses unsafe. The design of Dioxus *requires* unsafe (self-referential trees).

All of our test suite passes MIRI without errors.

Dioxus deals with arenas, lifetimes, asynchronous tasks, custom allocators, pinning, and a lot more foundational low-level work that is very difficult to implement with 0 unsafe.

If you don't want to use a crate that uses unsafe, then this crate is not for you.

However, we are always interested in decreasing the scope of the core VirtualDom to make it easier to review. We'd be happy to welcome PRs that can eliminate unsafe code while still upholding the numerous invariants required to execute certain features.

