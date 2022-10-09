# Concurrent mode

Concurrent mode provides a mechanism for building efficient asynchronous components. With this feature, components don't need to render immediately, and instead can schedule a future render by returning a future.

To make a component asynchronous, simply change its function signature to async.

```rust
fn Example(cx: Scope) -> Vnode {
    rsx!{ <div> "Hello world!" </div> }
}
```

becomes

```rust
async fn Example(cx: Scope) -> Vnode {
    rsx!{ <div> "Hello world!" </div> }
}
```

Now, logic in components can be awaited to delay updates of the component and its children. Like so:

```rust
async fn Example(cx: Scope) -> Vnode {
    let name = fetch_name().await;
    rsx!{ <div> "Hello {name}" </div> }
}

async fetch_name() -> String {
    // ...
}
```

This component will only schedule its render once the fetch is complete. However, we _don't_ recommend using async/await directly in your components.

Async is a notoriously challenging yet rewarding tool for efficient tools. If not careful, locking and unlocking shared aspects of the component's context can lead to data races and panics. If a shared resource is locked while the component is awaiting, then other components can be locked or panic when trying to access the same resource. These rules are especially important when references to shared global state are accessed using the context object's lifetime. If mutable references to data captured immutably by the context are taken, then the component will panic, causing confusion.

Instead, we suggest using hooks and future combinators that can safely utilize the safeguards of the component's Context when interacting with async tasks.

As part of our Dioxus hooks crate, we provide a data loader hook which pauses a component until its async dependencies are ready. This caches requests, reruns the fetch if dependencies have changed, and provides the option to render something else while the component is loading.

```rust
async fn ExampleLoader(cx: Scope) -> Vnode {
    /*
    Fetch, pause the component from rendering at all.

    The component is locked while waiting for the request to complete
    While waiting, an alternate component is scheduled in its place.

    This API stores the result on the Context object, so the loaded data is taken as reference.
    */
    let name: &Result<SomeStructure> = use_fetch_data("http://example.com/json", ())
                                        .place_holder(|cx| rsx!{<div> "loading..." </div>})
                                        .delayed_place_holder(1000, |cx| rsx!{ <div> "still loading..." </div>})
                                        .await;

    match name {
        Ok(name) => rsx! { <div> "Hello {something}" </div> },
        Err(e) => rsx! { <div> "An error occurred :(" </div>}
    }
}
```

```rust
async fn Example(cx: Scope) -> DomTree {
    // Diff this set between the last set
    // Check if we have any outstanding tasks?
    //
    // Eventually, render the component into the VDOM when the future completes
    <div>
        <Example />
    </div>

    // Render a div, queue a component
    // Render the placeholder first, then when the component is ready, then render the component
    <div>
        <Suspense placeholder={html!{<div>"Loading"</div>}}>
            <Example />
        </Suspense>
    </div>
}
```
