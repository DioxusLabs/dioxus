# Hello, World!

Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic reusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(ctx: Context<MyProps>) -> VNode {
    ctx.view(html! { 
        <div> "Hello {ctx.props().name}!" </div> 
    })
}
```

For functions to be valid components, they must take the `Context` object which is generic over some properties. The properties parameter must implement the `Properties` trait, which can be automatically derived. Whenever the input properties of a component changes, the function component will be re-ran and a new set of VNodes will be generated.
