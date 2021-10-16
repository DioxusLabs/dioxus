# Context API

```rust
// Create contexts available to children
// Only one context can be associated with any given component
// This is known as "exposed state". Children can access this context,
// but will not be automatically subscribed.
fn ContextCreate(cx: &mut Context<()>) -> DomTree {
    let context = cx.set_context(|| CustomContext::new());
    html! { <> {cx.children()} </> }
}

fn ContextRead(cx: &mut Context<()>) -> DomTree {
    // Panics if context is not available
    let some_cx = cx.get_context::<CustomContext>();
    let text = some_cx.select("some_selector");
    html! { <div> "{text}" </div> }
}

fn Subscription(cx: &mut Context<()>) -> DomTree {
    // Open a "port" on the component for actions to trigger a re-evaluation
    let subscription = cx.new_subscription();

    // A looping timer - the effect is re-called on every re-evaluation
    use_async_effect(cx, move || async {
        timer::new(2000).await;
        subscription.call();
    }, None);

    // A one-shot timer, the deps don't change so the effect only happens once
    use_async_effect_deps(cx, move || async {
        timer::new(2000).await;
        subscription.call();
    }, ());
}

// Mix subscriptions and context to make a simple Redux
fn use_global_state<T: UserContextTrait>(cx: &mut Context<()>) -> T {
    let some_cx = cx.get_context::<T>();
    let component_subscription = cx.new_subscription();
    some_cx.subscribe_component(component_subscription);
    some_cx
}

```
