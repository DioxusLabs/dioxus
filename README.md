# Dioxus: A concurrent, functional, virtual DOM for Rust

Dioxus is a new approach for creating performant cross platform user experiences in Rust. In Dioxus, the UI is represented by a tree of Virtual Nodes not bound to any renderer framework. Instead, external renderers can leverage Dioxus' virtual DOM and event system as a source of truth for rendering to a medium of their choice. Developers used to crafting react-based experiences should feel comfortable with Dioxus.

## Hello World
Dioxus should look and feel just like writing functional React components. In Dioxus, there are no class components with lifecycles. All state management is done via hooks. This encourages logic reusability and lessens the burden on Dioxus to maintain a non-breaking lifecycle API.

```rust
#[derive(Properties, PartialEq)]
struct MyProps {
    name: String
}

fn Example(ctx: &Context<MyProps>) -> VNode {
    html! { <div> "Hello {:?ctx.props().name}!" </div> }
}
```

Here, the `Context` object is used to access hook state, create subscriptions, and interact with the built-in context API. Props, children, and component APIs are accessible via the `Context` object. If using the functional component macro, it's possible to inline props into the function definition itself.

```rust
#[functional_component]
fn Example(ctx: &Context<{ name: String }>) -> VNode {
    html! { <div> "Hello {:?name}!" </div> }
}

// or

#[functional_component]
static Example: FC<{ name: String }> = |ctx| html! { <div> "Hello {:?name}!" </div> }; 
```

The final output of components must be a tree of VNodes. We provide an html macro for using JSX-style syntax to write these, though, you could use any macro, DSL, or templating engine. Work is being done on a terra template processor for existing templates.

## Features

Dioxus supports:
- Hooks
- Concurrent rendering
- Context subscriptions
- Isomorphism

## Concurrency

Dioxus, using React as a reference, provides the ability to have asynchronous components. With Dioxus, this is a valid component:

```rust
async fn user_data(ctx: &Context<()>) -> VNode {
    let Profile { name, birthday, .. } = use_context::<UserContext>(ctx).fetch_data().await;
    html! {
        <div>
            {"Hello, {:?name}!"}
            {if birthday === std::Instant::now() {html! {"Happy birthday!"}}}
        </div>
    }
}
```

Asynchronous components are powerful but can also be easy to misuse as they pause rendering for the component and its children. Refer to the concurrent guide for information on how to best use async components. 

## Examples
We use `diopack` to build and test webapps. This can run examples, tests, build web workers, launch development servers, bundle, and more. It's general purpose, but currently very tailored to Dioxus for liveview and bundling. If you've not used it before, `cargo install --path pacakages/diopack` will get it installed. 

Alternatively, `Trunk` works but can't run examples.

- tide_ssr: Handle an HTTP request and return an html body using the html! macro. `cargo run --example tide_ssr`
- simple_wasm: Simple WASM app that says hello. `diopack develop --example simple`

## Documentation
We have a pretty robust 

