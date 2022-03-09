# Global State

If your app has finally gotten large enough where passing values through the tree ends up polluting the intent of your code, then it might be time to turn to global state.

In Dioxus, global state is shared through the Context API. This guide will show you how to use the Context API to simplify state management.

## Provide Context and Consume Context

The simplest way of retrieving shared state in your app is through the Context API. The Context API allows you to provide and consume an item of state between two components.

Whenever a component provides a context, it is then accessible to any child components.

> Note: parent components cannot "reach down" and consume state from below their position in the tree.

The terminology here is important: `provide` context means that the component will expose state and `consume` context means that the child component can acquire a handle to state above it.

Instead of using keys or statics, Dioxus prefers the `NewType` pattern to search for parent state. This means each state you expose as a context should be its own unique type.

In practice, you'll have a component that exposes some state:


```rust
#[derive(Clone)]
struct Title(String);

fn app(cx: Scope) -> Element {
    cx.use_hook(|_| {
        cx.provide_context(Title("Hello".to_string()));
    });

    cx.render(rsx!{
        Child {}
    })
}
```

And then in our component, we can consume this state at any time:

```rust
fn Child(cx: Scope) -> Element {
    let name = cx.consume_context::<Title>();

    //
}
```

Note: calling "consume" state can be a rather expensive operation to perform during each render. Prefer to consume state within a `use_hook`:

```rust
fn Child(cx: Scope) -> Element {
    // cache our "consume_context" operation
    let name = cx.use_hook(|_| cx.consume_context::<Title>());
}
```

All `Context` must be cloned - the item will be cloned into each call of `consume_context`. To make this operation cheaper, consider wrapping your type in an `Rc` or `Arc`.


<!-- ## Coroutines

The `use_coroutine` hook  -->

<!-- # `use_context` and `use_context_provider`

These -->
