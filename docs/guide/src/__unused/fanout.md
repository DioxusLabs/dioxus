# Fanning Out

One of the most reliable state management patterns in large Dioxus apps is `fan-out`. The fan-out pattern is the ideal way to structure your app to maximize code reuse, testability, and deterministic rendering.

## The structure

With `fan-out`, our individual components at "leaf" position of our VirtualDom are "pure", making them easily reusable, testable, and deterministic. For instance, the "title" bar of our app might be a fairly complicated component.

```rust
#[derive(Props, PartialEq)]
struct TitlebarProps {
    title: String,
    subtitle: String,
}

fn Titlebar(cx: Scope<TitlebarProps>) -> Element {
    cx.render(rsx!{
        div {
            class: "titlebar"
            h1 { "{cx.props.title}" }
            h1 { "{cx.props.subtitle}" }
        }
    })
}
```

If we used global state like use_context or fermi, we might be tempted to inject our `use_read` directly into the component.

```rust
fn Titlebar(cx: Scope<TitlebarProps>) -> Element {
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    cx.render(rsx!{/* ui */})
}
```

For many apps - this is a fine pattern, especially if the component is a one-off. However, if we want to reuse the component outside of this app, then we'll start to run into issues where our contexts are unavailable.

## Fanning Out

To enable our titlebar component to be used across apps, we want to lift our atoms upwards and out of the Titlebar component. We would organize a bunch of other components in this section of the app to share some of the same state.

```rust
fn DocsiteTitlesection(cx: Scope) {
    let title = use_read(&cx, TITLE);
    let subtitle = use_read(&cx, SUBTITLE);

    let username = use_read(&cx, USERNAME);
    let points = use_read(&cx, POINTS);

    cx.render(rsx!{
        TitleBar { title: title, subtitle: subtitle }
        UserBar { username: username, points: points }
    })
}
```

This particular wrapper component unfortunately cannot be reused across apps. However, because it simply wraps other real elements, it doesn't have to. We are free to reuse our TitleBar and UserBar components across apps with ease. We also know that this particular component is plenty performant because the wrapper doesn't have any props and is always memoized. The only times this component re-renders is when any of the atoms change.

This is the beauty of Dioxus - we always know where our components are likely to be re-rendered. Our wrapper components can easily prevent any large re-renders by simply memoizing their components. This system might not be as elegant or precise as signal systems found in libraries like Sycamore or SolidJS, but it is quite ergonomic.
