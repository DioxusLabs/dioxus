# Spawning Futures

The `use_future` and `use_coroutine` hooks are useful if you want to unconditionally spawn the future. Sometimes, though, you'll want to only spawn a future in response to an event, such as a mouse click. For example, suppose you need to send a request when the user clicks a "log in" button. For this, you can use `cx.spawn`:

```rust
{{#include ../../examples/spawn.rs:spawn}}
```

> Note: `spawn` will always spawn a *new* future. You most likely don't want to call it on every render.

The future must be `'static` – so any values captured by the task cannot carry any references to `cx`, such as a `UseState`.

However, since you'll typically need a way to update the value of a hook, you can use `to_owned` to create a clone of the hook handle. You can then use that clone in the async closure.

To make this a bit less verbose, Dioxus exports the `to_owned!` macro which will create a binding as shown above, which can be quite helpful when dealing with many values.

```rust
{{#include ../../examples/spawn.rs:to_owned_macro}}
```

Calling `spawn` will give you a `JoinHandle` which lets you cancel or pause the future.

## Spawning Tokio Tasks

Sometimes, you might want to spawn a background task that needs multiple threads or talk to hardware that might block your app code. In these cases, we can directly spawn a Tokio task from our future. For Dioxus-Desktop, your task will be spawned onto Tokio's Multithreaded runtime:

```rust
{{#include ../../examples/spawn.rs:tokio}}
```
