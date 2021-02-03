/*
The Dioxus Virtual Dom integrates an event system and virtual nodes to create reactive user interfaces.

The Dioxus VDom uses the same underlying mechanics as Dodrio (double buffering, bump dom, etc).
Instead of making the allocator very obvious, we choose to parametrize over the DomTree trait. For our purposes,
the DomTree trait is simply an abstraction over a lazy dom builder, much like the iterator trait.

This means we can accept DomTree anywhere as well as return it. All components therefore look like this:
```ignore
function Component(ctx: Context<()>) -> impl DomTree {
    html! {<div> "hello world" </div>}
}
```
It's not quite as sexy as statics, but there's only so much you can do. The goal is to get statics working with the FC macro,
so types don't get in the way of you and your component writing. Fortunately, this is all generic enough to be split out
into its own lib (IE, lazy loading wasm chunks by function (exciting stuff!))

```ignore
#[fc] // gets translated into a function.
static Component: FC = |ctx| {
    html! {<div> "hello world" </div>}
}
```


This module includes all life-cycle related mechanics, including the virtual dom, scopes, properties, and lifecycles.
---
The VirtualDom is designed as so:

VDOM contains:
    - An arena of component scopes.
        - A scope contains
            - lifecycle data
            - hook data
    - Event queue
        - An event

A VDOM is
    - constructed from anything that implements "component"

A "Component" is anything (normally functions) that can be ran with a context to produce VNodes
    - Must implement properties-builder trait which produces a properties builder

A Context
    - Is a consumable struct
        - Made of references to properties
        - Holds a reference (lockable) to the underlying scope
        - Is partially threadsafe
*/
use crate::nodes::VNode;
use crate::prelude::*;
use bumpalo::Bump;
use generational_arena::Arena;
use std::future::Future;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    components: Arena<Scope>,

    /// Components generate lifecycle events
    event_queue: Vec<LifecycleEvent>,

    buffers: [Bump; 2],
}

impl VirtualDom {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root)
    }

    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    pub fn new_with_props<P: Properties>(root: FC<P>) -> Self {
        Self {
            components: Arena::new(),
            event_queue: vec![],
            buffers: [Bump::new(), Bump::new()],
        }
    }

    /// Pop an event off the even queue and process it
    pub fn progress_event() {}
}

/// The internal lifecycle event system is managed by these
/// All events need to be confused before swapping doms over
pub enum LifecycleEvent {
    Add {},
}

/// Anything that takes a "bump" and returns VNodes is a "DomTree"
/// This is used as a "trait alias" for function return types to look less hair
pub trait DomTree {
    fn render(self, b: &Bump) -> VNode;
}

/// Implement DomTree for the type returned by the html! macro.
/// This lets the caller of the static function evaluate the builder closure with its own bump.
/// It keeps components pretty and removes the need for the user to get too involved with allocation.
impl<F> DomTree for F
where
    F: FnOnce(&Bump) -> VNode,
{
    fn render(self, b: &Bump) -> VNode {
        self(b)
    }
}

/// The `Component` trait refers to any struct or funciton that can be used as a component
/// We automatically implement Component for FC<T>
pub trait Component {
    type Props: Properties;
    fn builder(&'static self) -> Self::Props;
}

// Auto implement component for a FC
// Calling the FC is the same as "rendering" it
impl<P: Properties> Component for FC<P> {
    type Props = P;

    fn builder(&self) -> Self::Props {
        todo!()
    }
}

/// The `Properties` trait defines any struct that can be constructed using a combination of default / optional fields.
/// Components take a "properties" object
pub trait Properties {
    fn new() -> Self;
}

// Auto implement for no-prop components
impl Properties for () {
    fn new() -> Self {
        ()
    }
}

// ============================================
// Compile Tests for FC/Component/Properties
// ============================================
#[cfg(test)]
mod fc_test {
    use super::*;

    // // Make sure this function builds properly.
    // fn test_static_fn<'a, P: Properties, F: DomTree>(b: &'a Bump, r: &FC<P, F>) -> VNode<'a> {
    //     let p = P::new(); // new props
    //     let c = Context { props: p }; // new context with props
    //     let g = r(&c); // calling function with context
    //     g.render(&b) // rendering closure with bump allocator
    // }

    // fn test_component(ctx: &Context<()>) -> impl DomTree {
    //     // todo: helper should be part of html! macro
    //     html! { <div> </div> }
    // }

    // fn test_component2(ctx: &Context<()>) -> impl DomTree {
    //     __domtree_helper(move |bump: &Bump| VNode::text("blah"))
    // }

    // #[test]
    // fn ensure_types_work() {
    //     // TODO: Get the whole casting thing to work properly.
    //     // For whatever reason, FC is not auto-implemented, depsite it being a static type
    //     let b = Bump::new();

    //     let g: FC<_, _> = test_component;
    //     let nodes0 = test_static_fn(&b, &g);
    //     // Happiness! The VNodes are now allocated onto the bump vdom

    //     let g: FC<_, _> = test_component2;
    //     let nodes1 = test_static_fn(&b, &g);
    // }
}

/// The Scope that wraps a functional component
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components
/// The actualy contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    hook_idx: i32,
    hooks: Vec<()>,
}

impl Scope {
    fn new<T>() -> Self {
        Self {
            hook_idx: 0,
            hooks: vec![],
        }
    }
}

pub struct HookState {}

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
/// This lets components schedule updates, integrate hooks, and expose their context via the context api.
///
/// Properties passed down from the parent component are also directly accessible via the exposed "props" field.
///
/// ```ignore
/// #[derive(Properties)]
/// struct Props {
///     name: String
///
/// }
///
/// fn example(ctx: &Context<Props>) -> VNode {
///     html! {
///         <div> "Hello, {ctx.props.name}" </div>
///     }
/// }
/// ```
// todo: force lifetime of source into T as a valid lifetime too
// it's definitely possible, just needs some more messing around
pub struct Context<'source, T> {
    /// Direct access to the properties used to create this component.
    pub props: &'source T,
}

impl<'a, T> Context<'a, T> {
    // impl<'a, T> Context<'a, T> {
    /// Access the children elements passed into the component
    pub fn children(&self) -> Vec<VNode> {
        todo!("Children API not yet implemented for component Context")
    }

    /// Access a parent context
    pub fn parent_context<C>(&self) -> C {
        todo!("Context API is not ready yet")
    }

    /// Create a subscription that schedules a future render for the reference component
    pub fn subscribe(&self) -> impl FnOnce() -> () {
        todo!("Subscription API is not ready yet");
        || {}
    }

    /// Take a lazy VNode structure and actually build it with the context of the VDom's efficient VNode allocator.
    ///
    /// ```ignore
    /// fn Component(ctx: Context<Props>) -> VNode {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_tree = html! {<div>"Hello World"</div>};
    ///     
    ///     // Actually build the tree and allocate it
    ///     ctx.view(lazy_tree)
    /// }
    ///```
    pub fn view(&self, v: impl FnOnce(&'a Bump) -> VNode<'a>) -> VNode<'a> {
        todo!()
    }

    /// Create a suspended component from a future.
    ///
    /// When the future completes, the component will be renderered
    pub fn suspend(
        &self,
        fut: impl Future<Output = impl FnOnce(&'a Bump) -> VNode<'a>>,
    ) -> VNode<'a> {
        todo!()
    }
}
