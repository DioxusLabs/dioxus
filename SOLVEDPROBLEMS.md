# Solved problems while building Dioxus

## FC Macro for more elegant components
Originally the syntax of the FC macro was meant to look like:

```rust
#[fc]
fn example(ctx: &Context<{ name: String }>) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}
```

`Context` was originally meant to be more obviously parameterized around a struct definition. However, while this works with rustc, this does not work well with Rust Analyzer. Instead, the new form was chosen which works with Rust Analyzer and happens to be more ergonomic. 

```rust
#[fc]
fn example(ctx: &Context, name: String) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}
```

## Anonymous Components

In Yew, the function_component macro turns a struct into a Trait `impl` with associated type `props`. Like so:

```rust
#[derive(Properties)]
struct Props {
    // some props
}

struct SomeComponent;
impl FunctionProvider for SomeComponent {
    type TProps = Props;

    fn run(&mut self, props: &Props) -> Html {
        // user's functional component goes here
    }
}

pub type SomeComponent = FunctionComponent<function_name>;
```
By default, the underlying component is defined as a "functional" implementation of the `Component` trait with all the lifecycle methods. In Dioxus, we don't allow components as structs, and instead take a "hooks-only" approach. However, we still need props. To get these without dealing with traits, we just assume functional components are modules. This lets the macros assume an FC is a module, and `FC::Props` is its props and `FC::component` is the component. Yew's method does a similar thing, but with associated types on traits.

Perhaps one day we might use traits instead.

The FC macro needs to work like this to generate a final module signature:

```rust
// "Example" can be used directly
// The "associated types" are just children of the module
// That way, files can just be components (yay, no naming craziness)
mod Example {
    // Associated metadata important for liveview
    static NAME: &'static str = "Example";

    struct Props {
        name: String
    }
    
    fn component(ctx: &Context<Props>) -> VNode {
        html! { <div> "Hello, {name}!" </div> }
    }
}

// or, Example.rs

static NAME: &'static str = "Example";

struct Props {
    name: String
}

fn component(ctx: &Context<Props>) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}
```

These definitions might be ugly, but the fc macro cleans it all up. The fc macro also allows some configuration

```rust
#[fc]
fn example(ctx: &Context, name: String) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}

// .. expands to 

mod Example {
    use super::*;
    static NAME: &'static str = "Example";
    struct Props {
        name: String
    }    
    fn component(ctx: &Context<Props>) -> VNode {
        html! { <div> "Hello, {name}!" </div> }
    }
}
```



## Live Components
Live components are a very important part of the Dioxus ecosystem. However, the goal with live components was to constrain their implementation purely to APIs available through Context (concurrency, context, subscription). 

From a certain perspective, live components are simply server-side-rendered components that update when their props change. Here's more-or-less how live components work:

```rust
#[fc]
static LiveFc: FC = |ctx, refresh_handler: impl FnOnce| {
    // Grab the "live context"
    let live_context = ctx.use_context::<LiveContext>();

    // Ensure this component is registered as "live"
    live_context.register_scope();

    // send our props to the live context and get back a future
    let vnodes = live_context.request_update(ctx);

    // Suspend the rendering of this component until the vnodes are finished arriving
    // Render them once available
    ctx.suspend(async move {
        let output = vnodes.await;

        // inject any listener handles (ie button clicks, views, etc) to the parsed nodes
        output[1].add_listener("onclick", refresh_handler);

        // Return these nodes
        // Nodes skip diffing and go straight to rendering
        output
    })
}
```

Notice that LiveComponent receivers (the client-side interpretation of a LiveComponent) are simply suspended components waiting for updates from the LiveContext (the context that wraps the app to make it "live"). 

## Allocation Strategy (ie incorporating Dodrio research)
----
The `VNodeTree` type is a very special type that allows VNodes to be created using a pluggable allocator. The html! macro creates something that looks like:

```rust
static Example: FC<()> = |ctx| {
    html! { <div> "blah" </div> }
};

// expands to...

static Example: FC<()> = |ctx| {
    // This function converts a Fn(allocator) -> VNode closure to a DomTree struct that will later be evaluated.
    html_macro_to_vnodetree(move |allocator| {
        let mut node0 = allocator.alloc(VElement::div);
        let node1 = allocator.alloc_text("blah");
        node0.children = [node1];
        node0
    })
};
```
At runtime, the new closure is created that captures references to `ctx`. Therefore, this closure can only be evaluated while `ctx` is borrowed and in scope. However, this closure can only be evaluated with an `allocator`. Currently, the global and Bumpalo allocators are available, though in the future we will add support for creating a VDom with any allocator or arena system (IE Jemalloc, wee-alloc, etc). The intention here is to allow arena allocation of VNodes (no need to box nested VNodes). Between diffing phases, the arena will be overwritten as old nodes are replaced with new nodes. This saves allocation time and enables bump allocators.

