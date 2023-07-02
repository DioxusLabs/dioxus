# UseEffect

[`use_effect`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html) provides a future that executes after the hooks have been applied.

Whenever the hooks [dependencies](#dependencies) change, the future will be re-evaluated. This is useful to syncrhonize with external events.

## Dependencies

You can make the future re-run when some value changes. For example, you might want to fetch a user's data only when the user id changes. You can provide a tuple of "dependencies" to the hook. It will automatically re-run the future when any of those dependencies change.

Example:

```rust, no_run
#[inline_props]
fn Profile(cx: Scope, id: usize) -> Element {
    let name = use_state(cx, || None);

    // Only fetch the user data when the id changes.
    use_effect(cx, (id,), |(id,)| {
        to_owned![name];
        async move {
            let user = fetch_user(id).await;
            name.set(user.name);
        }
    });

    let name = name.get().clone().unwrap_or("Loading...".to_string());

    render!(
        p { "{name}" }
    )
}

fn app(cx: Scope) -> Element {
    render!(Profile { id: 0 })
}
```
