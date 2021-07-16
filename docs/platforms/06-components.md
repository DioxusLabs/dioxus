## Components

Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic reusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(cx: Context<MyProps>) -> VNode {
    html! { <div> "Hello {cx.cx.name}!" </div> }
}
```

Here, the `Context` object is used to access hook state, create subscriptions, and interact with the built-in context API. Props, children, and component APIs are accessible via the `Context` object. The functional component macro makes life more productive by inlining props directly as function arguments, similar to how Rocket parses URIs.

```rust
// A very terse component!
#[fc]
fn Example(cx: Context, name: String) -> VNode {
    html! { <div> "Hello {name}!" </div> }
}

// or

#[functional_component]
pub static Example: FC = |cx, name: String| html! { <div> "Hello {name}!" </div> };
```

The final output of components must be a tree of VNodes. We provide an html macro for using JSX-style syntax to write these, though, you could use any macro, DSL, templating engine, or the constructors directly.
