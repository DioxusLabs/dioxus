# Lifting State and Fanning Out

Maintaining state local to components doesn't always work.

One of the most reliable state management patterns in large Dioxus apps is to `lift-up` and `fan-out`. Lifting up and fanning-out state is the ideal way to structure your app to maximize code reuse, testability, and deterministic rendering.


## Lifting State

When building complex apps with Dioxus, the best approach is to start by placing your state and an UI all within a single component. Once your component has passed a few hundred lines, then it might be worth refactoring it into a few smaller components.

Here, we're now challenged with how to share state between these various components.

Let's say we refactored our component to separate an input and a display.

```rust
fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        Title {}
        Input {}
    })
}
```

Whenever a value is inputted in our `Input` component, we need to somehow propagate those changes into the `Title` component.

A quick-and-dirty solution - which works for many apps - is to simply share a UseState between the two components.

```rust
fn app(cx: Scope) -> Element {
    let text = use_state(&cx, || "default".to_string());

    cx.render(rsx!{
        Title { text: text.clone() }
        Input { text: text.clone() }
    })
}
```

> Note: since we `Cloned` our `text` `UseState` handle, both `Title` and `Input` will be memoized.

Here, we've "lifted" state out of our `Input` component and brought it up to the closest shared ancestor. In our input component, we can directly use this UseState handle as if it had been defined locally:

```rust
#[inline_props]
fn Input(cx: Scope, text: UseState<String>) -> Element {
    cx.render(rsx!{
        input { oninput: move |evt| text.set(evt.value.clone()) }
    })
}
```
Similarly, our `Title` component would be straightforward:

```rust
#[inline_props]
fn Title(cx: Scope, text: UseState<String>) -> Element {
    cx.render(rsx!{
        h1 { "{text}" }
    })
}
```

For more complicated use cases, we can take advantage of the EventHandler coercion talked about before to pass in any callback. Recall that fields on components that start with "on" are automatically "upgraded" into an `EventHandler` at the call site.

This lets us abstract over the exact type of state being used to store the data.

For the `Input` component, we would simply add a new `oninput` field:

```rust
#[inline_props]
fn Input<'a>(cx: Scope<'a>, oninput: EventHandler<'a, String>) -> Element {
    cx.render(rsx!{
        input { oninput: move |evt| oninput.call(evt.value.clone()), }
    })
}
```

For our `Title` component, we could also abstract it to take any `&str`:

```rust
#[inline_props]
fn Title<'a>(cx: Scope<'a>, text: &'a str) -> Element {
    cx.render(rsx!{
        h1 { "{text}" }
    })
}
```

## Fanning Out

As your app grows and grows, you might need to start pulling in global state to avoid prop drilling. This tends to solve a lot of problems, but generates even more.

For instance, let's say we built a beautiful `Title` component. Instead of passing props in, we instead are using a `use_read` hook from Fermi.

```rust
fn Title(cx: Scope) -> Element {
    let title = use_read(&cx, TITLE);

    cx.render(rsx!{
        h1 { "{title}" }
    })
}
```

This is great - all is well in the world. We get precise updates, automatic memoization, and a solid abstraction. But, what happens when we want to reuse this component in another project? This component is now deeply intertwined with our global state - which might not be the same in another app.

In this case, we want to "lift" our global state out of "view" components.  With `lifting`, our individual components at "leaf" position of our VirtualDom are "pure", making them easily reusable, testable, and deterministic. For instance, the "title" bar of our app might be a fairly complicated component.


To enable our title component to be used across apps, we want to lift our atoms upwards and out of the Title component. We would organize a bunch of other components in this section of the app to share some of the same state.

```rust
fn DocsiteTitlesection(cx: Scope) {
    // Use our global state in a wrapper component
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    let username = use_read(&cx, USERNAME);
    let points = use_read(&cx, POINTS);

    // and then pass our global state in from the outside
    cx.render(rsx!{
        Title { title: title.clone(), subtitle: subtitle.clone() }
        User { username: username.clone(), points: points.clone() }
    })
}
```

This particular wrapper component unfortunately cannot be reused across apps. However, because it simply wraps other real elements, it doesn't have to. We are free to reuse our TitleBar and UserBar components across apps with ease. We also know that this particular component is plenty performant because the wrapper doesn't have any props and is always memoized. The only times this component re-renders is when any of the atoms change.

This is the beauty of Dioxus - we always know where our components are likely to be re-rendered. Our wrapper components can easily prevent any large re-renders by simply memoizing their components. This system might not be as elegant or precise as signal systems found in libraries like Sycamore or SolidJS, but it is quite ergonomic.
