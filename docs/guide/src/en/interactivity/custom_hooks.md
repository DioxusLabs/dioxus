# Custom Hooks

Hooks are a great way to encapsulate business logic. If none of the existing hooks work for your problem, you can write your own.

When writing your hook, you can make a function that accepts `cx: &ScopeState` as a parameter to accept a scope with any Props.

## Composing Hooks

To avoid repetition, you can encapsulate business logic based on existing hooks to create a new hook.

For example, if many components need to access an `AppSettings` struct, you can create a "shortcut" hook:

```rust, no_run
{{#include ../../../examples/hooks_composed.rs:wrap_context}}
```

Or if you want to wrap a hook that persists reloads with the storage API, you can build on top of the use_ref hook to work with mutable state:

```rust, no_run
{{#include ../../../examples/hooks_composed.rs:use_storage}}
```

## Custom Hook Logic

You can use [`cx.use_hook`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.use_hook) to build your own hooks. In fact, this is what all the standard hooks are built on!

`use_hook` accepts a single closure for initializing the hook. It will be only run the first time the component is rendered. The return value of that closure will be used as the value of the hook – Dioxus will take it, and store it for as long as the component is alive. On every render (not just the first one!), you will get a reference to this value.

> Note: You can implement [`Drop`](https://doc.rust-lang.org/std/ops/trait.Drop.html) for your hook value – it will be dropped then the component is unmounted (no longer in the UI)

Inside the initialization closure, you will typically make calls to other `cx` methods. For example:

- The `use_state` hook tracks state in the hook value, and uses [`cx.schedule_update`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.schedule_update) to make Dioxus re-render the component whenever it changes.

Here is a simplified implementation of the `use_state` hook:

```rust, no_run
{{#include ../../../examples/hooks_custom_logic.rs:use_state}}
```

- The `use_context` hook calls [`cx.consume_context`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.ScopeState.html#method.consume_context) (which would be expensive to call on every render) to get some context from the scope

Here is an implementation of the `use_context` and `use_context_provider` hooks:

```rust, no_run
{{#include ../../../examples/hooks_custom_logic.rs:use_context}}
```

## Hook Anti-Patterns

When writing a custom hook, you should avoid the following anti-patterns:

- !Clone Hooks: To allow hooks to be used within async blocks, the hooks must be Clone. To make a hook clone, you can wrap data in Rc or Arc and avoid lifetimes in hooks.

This version of use_state may seem more efficient, but it is not cloneable:

```rust, no_run
{{#include ../../../examples/hooks_anti_patterns.rs:non_clone_state}}
```

If we try to use this hook in an async block, we will get a compile error:

```rust, no_run
fn FutureComponent(cx: &ScopeState) -> Element {
    let my_state = my_use_state(cx, || 0);
    cx.spawn({
        to_owned![my_state];
        async move {
            my_state.set(1);
        }
    });

    todo!()
}
```

But with the original version, we can use it in an async block:

```rust, no_run
fn FutureComponent(cx: &ScopeState) -> Element {
    let my_state = use_state(cx, || 0);
    cx.spawn({
        to_owned![my_state];
        async move {
            my_state.set(1);
        }
    });

    todo!()
}
```
