# UseEffect

[`use_effect`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_effect.html) lets you run a callback that returns a future, which will be re-run when it's [dependencies](#dependencies) change. This is useful to syncrhonize with external events.

## Dependencies

You can make the callback re-run when some value changes. For example, you might want to fetch a user's data only when the user id changes. You can provide a tuple of "dependencies" to the hook. It will automatically re-run it when any of those dependencies change.

## Example

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

    // Because the dependencies are empty, this will only run once.
    // An empty tuple is always equal to an empty tuple.
    use_effect(cx, (), |()| async move {
        println!("Hello, World!");
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
