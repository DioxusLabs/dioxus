# Subscriptions

Yew subscriptions are used to schedule update for components into the future. The `Context` object can create subscriptions:

```rust
fn Component(cx: Component) -> DomTree {
    let update = cx.schedule();

    // Now, when the subscription is called, the component will be re-evaluated
    update.consume();
}
```

Whenever a component's subscription is called, the component will then re-evaluated. You can consider the input properties of
a component to be just another form of subscription. By default, the Dioxus component system automatically diffs a component's props
when the parent function is called, and if the props are different, the child component's subscription is called.

The subscription API exposes this functionality allowing hooks and state management solutions the ability to update components whenever
some state or event occurs outside of the component. For instance, the `use_context` hook uses this to subscribe components that use a
particular context.

```rust
fn use_context<I>(cx: Scope<T>) -> I {

}







```
