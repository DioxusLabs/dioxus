# Context API



```rust
// Create contexts available to children
// Only one context can be associated with any given component
// This is known as "exposed state". Children can access this context,
// but will not be automatically subscribed.
fn ContextCreate(ctx: &mut Context<()>) -> VNode {
    let context = ctx.set_context(|| CustomContext::new());
    html! { <> {ctx.children()} </> }
}

fn ContextRead(ctx: &mut Context<()>) -> VNode {
    // Panics if context is not available
    let some_ctx = ctx.get_context::<CustomContext>();
    let text = some_ctx.select("some_selector");
    html! { <div> "{text}" </div> }
}

fn Subscription(ctx: &mut Context<()>) -> VNode {
    // Open a "port" on the component for actions to trigger a re-evaluation
    let subscription = ctx.new_subscription();

    // A looping timer - the effect is re-called on every re-evaluation
    use_async_effect(ctx, move || async {
        timer::new(2000).await;
        subscription.call();
    }, None);

    // A one-shot timer, the deps don't change so the effect only happens once
    use_async_effect_deps(ctx, move || async {
        timer::new(2000).await;
        subscription.call();
    }, ());
}

// Mix subscriptions and context to make a simple Redux
fn use_global_state<T: UserContextTrait>(ctx: &mut Context<()>) -> T {
    let some_ctx = ctx.get_context::<T>();
    let component_subscription = ctx.new_subscription();
    some_ctx.subscribe_component(component_subscription);
    some_ctx
}

```
