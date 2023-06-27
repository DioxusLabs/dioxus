# UseEffect

[`use_effect`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html) provides a future that executes after the hooks have been applied.

Whenever the hooks dependencies change, the future will be re-evaluated. This is useful to syncrhonize with external events.

If a future is pending when the dependencies change, the previous future will be allowed to continue

> The `dependencies` is tuple of references to values that are `PartialEq + Clone`.

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
