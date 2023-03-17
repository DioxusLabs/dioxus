# Custom Hooks

Hooks are a great way to encapsulate business logic. If none of the existing hooks work for your problem, you can write your own.

## Composing Hooks

To avoid repetition, you can encapsulate business logic based on existing hooks to create a new hook.

For example, if many components need to access an `AppSettings` struct, you can create a "shortcut" hook:

```rust
{{#include ../../../examples/hooks_composed.rs:wrap_context}}
```

## Custom Hook Logic

You can use [`cx.use_hook`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.use_hook) to build your own hooks. In fact, this is what all the standard hooks are built on!

`use_hook` accepts a single closure for initializing the hook. It will be only run the first time the component is rendered. The return value of that closure will be used as the value of the hook – Dioxus will take it, and store it for as long as the component is alive. On every render (not just the first one!), you will get a reference to this value.

> Note: You can implement [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) for your hook value – it will be dropped then the component is unmounted (no longer in the UI)

Inside the initialization closure, you will typically make calls to other `cx` methods. For example:

- The `use_state` hook tracks state in the hook value, and uses [`cx.schedule_update`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.schedule_update) to make Dioxus re-render the component whenever it changes.
- The `use_context` hook calls [`cx.consume_context`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.consume_context) (which would be expensive to call on every render) to get some context from the scope
