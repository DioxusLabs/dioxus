# UseEffect

[`use_effect`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html) provides a future that executes after the hooks have been applied.

Whenever the hooks [dependencies](#dependencies) change, the future will be re-evaluated. This is useful to syncrhonize with external events.


## Dependencies

You might want to call `use_effect` only when some value changes. For example, you might want to fetch a user's data only when the user id changes. You can provide a tuple of "dependencies" to the hook. It will automatically re-run the future when any of those dependencies change.

Example:

```rust, no_run
#[inline_props]
fn Profile(cx: Scope, id: &str) -> Element {
    let name = use_state(cx, || "Default name");

    // Only fetch the user data when the id changes.
    use_effect(cx, (id,), |(id,)| async move {
        let user = fetch_user(id).await;
        name.set(user.name);
    });

    render!(
        p { "{name}" }
    )
}

fn app(cx: Scope) -> Element {
    render!(
        Profile { id: "dioxusLabs" }
    )
}
```