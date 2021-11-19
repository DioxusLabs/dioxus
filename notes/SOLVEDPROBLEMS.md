# Solved problems while building Dioxus

focuses:

- ergonomics
- render agnostic
- remote coupling
- memory efficient
- concurrent
- global context
- scheduled updates
-

## FC Macro for more elegant components

Originally the syntax of the FC macro was meant to look like:

```rust
#[fc]
fn example(cx: &Context<{ name: String }>) -> DomTree {
    html! { <div> "Hello, {name}!" </div> }
}
```

`Context` was originally meant to be more obviously parameterized around a struct definition. However, while this works with rustc, this does not work well with Rust Analyzer. Instead, the new form was chosen which works with Rust Analyzer and happens to be more ergonomic.

```rust
#[fc]
fn example(cx: &Context, name: String) -> DomTree {
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

By default, the underlying component is defined as a "functional" implementation of the `Component` trait with all the lifecycle methods. In Dioxus, we don't allow components as structs, and instead take a "hooks-only" approach. However, we still need cx. To get these without dealing with traits, we just assume functional components are modules. This lets the macros assume an FC is a module, and `FC::Props` is its props and `FC::component` is the component. Yew's method does a similar thing, but with associated types on traits.

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

    fn component(cx: &Context<Props>) -> DomTree {
        html! { <div> "Hello, {name}!" </div> }
    }
}

// or, Example.rs

static NAME: &'static str = "Example";

struct Props {
    name: String
}

fn component(cx: &Context<Props>) -> DomTree {
    html! { <div> "Hello, {name}!" </div> }
}
```

These definitions might be ugly, but the fc macro cleans it all up. The fc macro also allows some configuration

```rust
#[fc]
fn example(cx: &Context, name: String) -> DomTree {
    html! { <div> "Hello, {name}!" </div> }
}

// .. expands to

mod Example {
    use super::*;
    static NAME: &'static str = "Example";
    struct Props {
        name: String
    }
    fn component(cx: &Context<Props>) -> DomTree {
        html! { <div> "Hello, {name}!" </div> }
    }
}
```

## Live Components

Live components are a very important part of the Dioxus ecosystem. However, the goal with live components was to constrain their implementation purely to APIs available through Context (concurrency, context, subscription).

From a certain perspective, live components are simply server-side-rendered components that update when their props change. Here's more-or-less how live components work:

```rust
#[fc]
static LiveFc: FC = |cx, refresh_handler: impl FnOnce| {
    // Grab the "live context"
    let live_context = cx.use_context::<LiveContext>();

    // Ensure this component is registered as "live"
    live_context.register_scope();

    // send our props to the live context and get back a future
    let vnodes = live_context.request_update(cx);

    // Suspend the rendering of this component until the vnodes are finished arriving
    // Render them once available
    cx.suspend(async move {
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

---

The `VNodeTree` type is a very special type that allows VNodes to be created using a pluggable allocator. The html! macro creates something that looks like:

```rust
pub static Example: FC<()> = |cx, props|{
    html! { <div> "blah" </div> }
};

// expands to...

pub static Example: FC<()> = |cx, props|{
    // This function converts a Fn(allocator) -> DomTree closure to a VNode struct that will later be evaluated.
    html_macro_to_vnodetree(move |allocator| {
        let mut node0 = allocator.alloc(VElement::div);
        let node1 = allocator.alloc_text("blah");
        node0.children = [node1];
        node0
    })
};
```

At runtime, the new closure is created that captures references to `cx`. Therefore, this closure can only be evaluated while `cx` is borrowed and in scope. However, this closure can only be evaluated with an `allocator`. Currently, the global and Bumpalo allocators are available, though in the future we will add support for creating a VDom with any allocator or arena system (IE Jemalloc, wee-alloc, etc). The intention here is to allow arena allocation of VNodes (no need to box nested VNodes). Between diffing phases, the arena will be overwritten as old nodes are replaced with new nodes. This saves allocation time and enables bump allocators.

## Context and lifetimes

We want components to be able to fearlessly "use_context" for use in state management solutions.

However, we cannot provide these guarantees without compromising the references. If a context mutates, it cannot lend out references.

Functionally, this can be solved with UnsafeCell and runtime dynamics. Essentially, if a context mutates, then any affected components would need to be updated, even if they themselves aren't updated. Otherwise, a reference would be pointing at data that could have potentially been moved.

To do this safely is a pretty big challenge. We need to provide a method of sharing data that is safe, ergonomic, and that fits the abstraction model.

Enter, the "ContextGuard".

The "ContextGuard" is very similar to a Ref/RefMut from the RefCell implementation, but one that derefs into actual underlying value.

However, derefs of the ContextGuard are a bit more sophisticated than the Ref model.

For RefCell, when a Ref is taken, the RefCell is now "locked." This means you cannot take another `borrow_mut` instance while the Ref is still active. For our purposes, our modification phase is very limited, so we can make more assumptions about what is safe.

1. We can pass out ContextGuards from any use of use_context. These don't actually lock the value until used.
2. The ContextGuards only lock the data while the component is executing and when a callback is running.
3. Modifications of the underlying context occur after a component is rendered and after the event has been run.

With the knowledge that usage of ContextGuard can only be achieved in a component context and the above assumptions, we can design a guard that prevents any poor usage but also is ergonomic.

As such, the design of the ContextGuard must:

- be /copy/ for easy moves into closures
- never point to invalid data (no dereferencing of raw pointers after movable data has been changed (IE a vec has been resized))
- not allow references of underlying data to leak into closures

To solve this, we can be clever with lifetimes to ensure that any data is protected, but context is still ergonomic.

1. As such, deref context guard returns an element with a lifetime bound to the borrow of the guard.
2. Because we cannot return locally borrowed data AND we consume context, this borrow cannot be moved into a closure.
3. ContextGuard is _copy_ so the guard itself can be moved into closures
4. ContextGuard derefs with its unique lifetime _inside_ closures
5. Derefing a ContextGuard evaluates the underlying selector to ensure safe temporary access to underlying data

```rust
struct ExampleContext {
    // unpinnable objects with dynamic sizing
    items: Vec<String>
}

fn Example<'src>(cx: Context<'src, ()>) -> DomTree<'src> {
    let val: &'b ContextGuard<ExampleContext> = (&'b cx).use_context(|context: &'other ExampleContext| {
        // always select the last element
        context.items.last()
    });

    let handler1 = move |_| println!("Value is {}", val); // deref coercion performed here for printing
    let handler2 = move |_| println!("Value is {}", val); // deref coercion performed here for printing

    cx.render(html! {
        <div>
            <button onclick={handler1}> "Echo value with h1" </button>
            <button onclick={handler2}> "Echo value with h2" </button>
            <div>
                <p> "Value is: {val}" </p>
            </div>
        </div>
    })
}
```

A few notes:

- this does _not_ protect you from data races!!!
- this does _not_ force rendering of components
- this _does_ protect you from invalid + UB use of data
- this approach leaves lots of room for fancy state management libraries
- this approach is fairly quick, especially if borrows can be cached during usage phases

## Concurrency

For Dioxus, concurrency is built directly into the VirtualDOM lifecycle and event system. Suspended components prove "no changes" while diffing, and will cause a lifecycle update later. This is considered a "trigger" and will cause targeted diffs and re-renders. Renderers will need to await the Dioxus suspense queue if they want to process these updates. This will typically involve joining the suspense queue and event listeners together like:

```rust
// wait for an even either from the suspense queue or our own custom listener system
let (left, right) = join!(vdom.suspense_queue, self.custom_event_listener);
```

LiveView is built on this model, and updates from the WebSocket connection to the host server are treated as external updates. This means any renderer can feed targeted EditLists (the underlying message of this event) directly into the VirtualDOM.

## Execution Model

<!-- todo -->

## Diffing

Diffing is an interesting story. Since we don't re-render the entire DOM, we need a way to patch up the DOM without visiting every component. To get this working, we need to think in cycles, queues, and stacks. Most of the original logic is pulled from Dodrio as Dioxus and Dodrio share much of the same DNA.

When an event is triggered, we find the callback that installed the listener and run it. We then record all components affected by the running of the "subscription" primitive. In practice, many hooks will initiate a subscription, so it is likely that many components throughout the entire tree will need to be re-rendered. For each component, we attach its index and the type of update it needs.

In practice, removals trump prop updates which trump subscription updates. Therefore, we only process updates where props are directly changed first, as this will likely flow into child components.

Roughly, the flow looks like:

- Process the initiating event
- Mark components affected by the subscription API (the only way of causing forward updates)
- Descend from the root into children, ignoring those not affected by the subscription API. (walking the tree until we hit the first affected component, or choosing the highest component)
- Run this component and then immediately diff its output, marking any children that also need to be updated and putting them into the immediate queue
- Mark this component as already-ran and remove it from the need_to_diff list, instead moving it into the "already diffed list"
- Run the marked children until the immediate queue is empty

```rust
struct DiffMachine {
    immediate_queue: Vec<Index>,
    diffed: HashSet<Index>,
    need_to_diff: HashSet<Index>
    marked_for_removal: Vec<Index>
}
```

On the actual diffing level, we're using the diffing algorithm pulled from Dodrio, but plan to move to a dedicated crate that implements Meyers/Patience for us. During the diffing phase, we track our current position using a "Traversal" which implements the "MoveTo". When "MoveTo" is combined with "Edit", it is possible for renderers to fully interpret a series of Moves and Edits together to update their internal node structures for rendering.

## Patch Stream

One of the most important parts of Dioxus is the ability to stream patches from server to client. However, this inherently has challenges where raw VNodes attach listeners to themselves, and are therefore not serializable.

### How do properties work?

How should properties passing work? Should we directly call the child? Should we box the props? Should we replace the pops inside the box?

Here's (generally) my head is at:

Components need to store their props on them if they want to be updated remotely. These props _can_ be updated after the fact.

Perf concerns:
unnecessary function runs - list-y components - hook calls? - making vnodes?

Does any of this matter?
Should we just run any component we see, immediately and imperatively? That will cause checks throughout the whole tree, no matter where the update occurred

https://calendar.perfplanet.com/2013/diff/

Here's how react does it:

Any "dirty" node causes an entire subtree render. Calling "setState" at the very top will cascade all the way down. This is particularly bad for this component design:

```rust
static APP: FC<()> = |cx, props|{
    let title = use_context(Title);
    cx.render(html!{
        <div>
            <h1> "{title}"</h1>
            <HeavyList /> // VComponent::new(|| (FC, PropsForFc)) -> needs a context to immediately update the component's props imperatively? store the props in a box on bump? store the props on the child?
            // if props didnt change, then let the refernece stay invalid?.... no, cant do that, bump gets reset
            // immediately update props on the child component if it can be found? -> interesting, feels wrong, but faster, at the very least.
            // can box on bump for the time being (fast enough), and then move it over? during the comparison phase? props only need to matter
            // cant downcast (can with transmute, but yikes)
            // how does chain borrowing work? a -> b -> c -> d
            // if b gets marked as dirty, then c and d are invalidated (semantically, UB, but not *bad* UB, just data races)
            // make props static? -> easy to move, gross to use
            //
            // treat like a context selector?
            // use_props::<P>(2)
            // child_props: Map<Scope, Box<dyn Props>>
            // vs children: BTreeSet<Scope> -> to get nth
        </div>
    })
};
static HEAVY_LIST: FC<()> = |cx, props|{
    cx.render({
        {0.100.map(i => <BigElement >)}
    })
};
```

An update to the use_context subscription will mark the node as dirty. The node then is forced to re-analyze HeavyList, even though HeavyList did not change. We should automatically implement this suppression knowing that props are immutable and can be partialeq.

## FC Layout

The FC layout was altered to make life easier for us inside the VirtualDom. The "view" function returns an unbounded VNode object. Calling the "view" function is unsafe under the hood, but prevents lifetimes from leaking out of the function call. Plus, it's easier to write. Because there are no lifetimes on the output (occur purely under the hood), we can escape needing to annotate them.

```rust
fn component(cx: Context, props: &Props) -> DomTree {

}
```

The VNode object purely represents a viewable "key". It also forces components to use the "view" function as there is no other way to generate the VNode object. Because the VNode is a required type of FC, we can guarantee the same usage and flow patterns for all components.

## Events

Events are finally in! To do events properly, we are abstracting over the event source with synthetic events. This forces 3rd party renderers to create the appropriate cross-platform event

## Optional Props on Components

A major goal here is ergonomics. Any field that is Option<T> should default to none.

```rust

rsx! {
    Example { /* args go here */ a: 10, b: 20 }
}


```

```rust
#[derive(Properties)]
struct Props {

}

static Component: FC<Props> = |cx, props|{

}
```

or

```rust
#[fc]
static Component: FC = |cx, name: &str| {

}
```

## Noderefs

How do we resolve noderefs in a world of patches? Patches _must_ be serializable, so if we do something like `Option<&RefCell<Slot>>`, then that must serialize as _something_ to indicate to a remote host that access to the node itself is desired. Our `Slot` type will need to be somewhat abstract.

If we add a new patch type called "BindRef" we could do something like:

```rust
enum Patch {
    //...
    BindAsRef { raw_node: &RefCell<Option<Slot>> }
}
```

```rust
let node_ref = use_node_ref(&cx);
use_effect(&cx, || {

}, []);
div { ref: node_ref,
    "hello me"
    h3 {"yo dom"}
}
```

refs only work when you're native to the platform. it doesn't make sense to gain a ref when you're not native.

## In-sync or separate?

React makes refs - and protection against dom manipulation - work by modifying the real dom while diffing the virtual dom. This lets it bind real dom elements to the virtual dom elements. Dioxus currently does not do this, instead creating a list of changes for an interpreter to apply once diffing has completed.

This behavior fit dodrio well as all dom manipulations would occur batched. The original intention for this approach was to make it faster to read out of Wasm and into JS. Dodrio is essentially performing the Wasm job that Wasm<->JS for strings does. In theory, this particular optimization is not necessary.

https://github.com/fitzgen/dodrio/issues/77

This issue/pr on the dodrio repository points to a future where elements are held on to by the virtualdom.

Can we solve events, refs, and protection against 3rd party dom mutation all in one shot?

I think we can....

every node gets a globally unique ID

abstract the real dom

```rust

struct VirtualDom<Dom: RealDom>

trait RealDom {
    type Node: RealNode;
    fn get_node(&self, id: u32) -> &Self::Node;
    fn get_node_mut(&mut self, id: u32) -> &mut Self::Node;
    fn replace_node();
    fn create_text_node();
    fn create_element();
    fn create_element_ns();
}

trait RealNode {
    fn add_listener(&mut self, event: &str);
    fn set_inner_text(&mut self, text: &str);
    fn set_attr(&mut self, name, value);
    fn set_class(&mut self);
    fn remove_attr(&mut self);
    // We can't have a generic type in trait objects, so instead we provide the inner as Any
    fn raw_node_as_any_mut(&mut self) -> &mut dyn Any;
}

impl VirtualDom<Dom: RealDom> {
    fn diff<Dom: RealDom>() {

    }
}
enum VNode<'bump, 'realdom, RealDom> {
    VElement {
        real: &RealDom::Node
    }
    VText {
        real: &RealDom::Node
    }
}


fn main() {
    let mut real_dom = websys::Document();
    let virtual_dom = Dioxus::VirtualDom::new();

    virtual_dom.rebuild(&mut real_dom);

    loop {
        let event = switch! {
            real_dom.events.await => event,
            virtual_dom.inner_events.await => event
        };

        virtual_dom.apply_event(&mut real_dom, event);
    }
}

```
