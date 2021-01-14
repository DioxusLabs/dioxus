# Dioxus: A concurrent, functional, arena-allocated VDOM implementation for creating UIs in Rust

Dioxus is a new approach for creating performant cross platform user experiences in Rust. In Dioxus, the UI is represented by a tree of Virtual Nodes not bound to any renderer framework. Instead, external renderers can leverage Dioxus' virtual DOM and event system as a source of truth for rendering to a medium of their choice. Developers used to crafting react-based experiences should feel comfortable with Dioxus.

## Hello World
Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic resusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(ctx: Context<MyProps>) -> VNode {
    html! { <div> "Hello {ctx.props().name}!" </div> }
}
```










To build user interfaces, you must provide a way of creating VNodes. We provide a macro `dioxus-rsx` which makes it easy to drop in html templates and event listeners to make interactive user experiences.

Inspired by React's Concurrent Mode, components in Dioxus are asynchronous by default. When components need to load asynchronous data, their rendering will be halted until ready, leading to fewer DOM updates and greater performance. External crates can tap into this system using futures to craft useful transition-based hooks.

Rules of Dioxus:
- Every component is asynchronous
- Components will queued when completed
- 

Dioxus supports:
- Hooks
- Concurrent rendering
- Context subscriptions


