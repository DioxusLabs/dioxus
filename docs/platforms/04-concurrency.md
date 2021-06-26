## Concurrency

In Dioxus, VNodes are asynchronous and can their rendering can be paused at any time by awaiting a future. Hooks can combine this functionality with the Context and Subscription APIs to craft dynamic and efficient user experiences.

```rust
fn user_data(cx: Context<()>) -> VNode {
    // Register this future as a task
    use_suspense(cx, async {
        // Continue on with the component as usual, waiting for data to arrive
        let Profile { name, birthday, .. } = fetch_data().await;
        html! {
            <div>
                {"Hello, {name}!"}
                {if birthday === std::Instant::now() {html! {"Happy birthday!"}}}
            </div>
        }
    })
}
```

Asynchronous components are powerful but can also be easy to misuse as they pause rendering for the component and its children. Refer to the concurrent guide for information on how to best use async components.
