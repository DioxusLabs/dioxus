# Spawning Futures

The `use_future` and `use_coroutine` hooks are useful if you want to unconditionally spawn the future. Sometimes, though, you'll want to only spawn a future in response to an event, such as a mouse click. For example, suppose you need to send a request when the user clicks a "log in" button. For this, you can use `cx.spawn`:

```rust
{{#include ../../../examples/spawn.rs:spawn}}
```

> Note: `spawn` will always spawn a *new* future. You most likely don't want to call it on every render.

Calling `spawn` will give you a `JoinHandle` which lets you cancel or pause the future.

## Spawning Tokio Tasks

Sometimes, you might want to spawn a background task that needs multiple threads or talk to hardware that might block your app code. In these cases, we can directly spawn a Tokio task from our future. For Dioxus-Desktop, your task will be spawned onto Tokio's Multithreaded runtime:

```rust
{{#include ../../../examples/spawn.rs:tokio}}
```
