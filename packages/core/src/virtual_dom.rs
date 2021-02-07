/*
The Dioxus Virtual Dom integrates an event system and virtual nodes to create reactive user interfaces.

The Dioxus VDom uses the same underlying mechanics as Dodrio (double buffering, bump dom, etc).
Instead of making the allocator very obvious, we choose to parametrize over the DomTree trait. For our purposes,
the DomTree trait is simply an abstraction over a lazy dom builder, much like the iterator trait.

This means we can accept DomTree anywhere as well as return it. All components therefore look like this:
```ignore
function Component(ctx: Context<()>) -> VNode {
    ctx.view(html! {<div> "hello world" </div>})
}
```
It's not quite as sexy as statics, but there's only so much you can do. The goal is to get statics working with the FC macro,
so types don't get in the way of you and your component writing. Fortunately, this is all generic enough to be split out
into its own lib (IE, lazy loading wasm chunks by function (exciting stuff!))

```ignore
#[fc] // gets translated into a function.
static Component: FC = |ctx| {
    ctx.view(html! {<div> "hello world" </div>})
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
use generational_arena::{Arena, Index};
use std::{
    any::TypeId,
    cell::{RefCell, UnsafeCell},
    future::Future,
    sync::atomic::AtomicUsize,
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom<P: Properties> {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    components: Arena<Scope>,

    base_scope: Index,

    /// Components generate lifecycle events
    event_queue: Vec<LifecycleEvent>,

    buffers: [Bump; 2],

    selected_buf: u8,

    root_props: P,
}

/// Implement VirtualDom with no props for components that initialize their state internal to the VDom rather than externally.
impl VirtualDom<()> {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }
}

/// Implement the VirtualDom for any Properties
impl<P: Properties + 'static> VirtualDom<P> {
    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    pub fn new_with_props(root: FC<P>, root_props: P) -> Self {
        // 1. Create the buffers
        // 2. Create the component arena
        // 3. Create the base scope (can never be removed)
        // 4. Create the lifecycle queue
        // 5. Create the event queue
        let buffers = [Bump::new(), Bump::new()];

        // Arena allocate all the components
        // This should make it *really* easy to store references in events and such
        let mut components = Arena::new();

        // Create a reference to the component in the arena
        let base_scope = components.insert(Scope::new(root));

        // Create an event queue with a mount for the base scope
        let event_queue = vec![];

        Self {
            components,
            base_scope,
            event_queue,
            buffers,
            root_props,
            selected_buf: 0,
        }
    }

    /// Pop an event off the even queue and process it
    pub fn progress(&mut self) -> Result<(), ()> {
        let LifecycleEvent { index, event_type } = self.event_queue.pop().ok_or(())?;

        let scope = self.components.get(index).ok_or(())?;

        match event_type {
            // Component needs to be mounted to the virtual dom
            LifecycleType::Mount {} => {
                // todo! run the FC with the bump allocator
                // Run it with its properties
            }

            // The parent for this component generated new props and the component needs update
            LifecycleType::PropsChanged {} => {}

            // Component was successfully mounted to the dom
            LifecycleType::Mounted {} => {}

            // Component was removed from the DOM
            // Run any destructors and cleanup for the hooks and the dump the component
            LifecycleType::Removed {} => {
                let f = self.components.remove(index);
            }

            // Component was moved around in the DomTree
            // Doesn't generate any event but interesting to keep track of
            LifecycleType::Moved {} => {}

            // Component was messaged via the internal subscription service
            LifecycleType::Messaged => {}
        }

        Ok(())
    }

    /// Update the root props, causing a full event cycle
    pub fn update_props(&mut self, new_props: P) {}

    /// Run through every event in the event queue until the events are empty.
    /// Function is asynchronous to allow for async components to finish their work.
    pub async fn progess_completely() {}

    /// Create a new context object for a given component and scope
    fn new_context<T: Properties>(&self) -> Context<T> {
        todo!()
    }

    /// Stop writing to the current buffer and start writing to the new one.
    /// This should be done inbetween CallbackEvent handling, but not between lifecycle events.
    pub fn swap_buffers(&mut self) {}
}

pub struct LifecycleEvent {
    pub index: Index,
    pub event_type: LifecycleType,
}
impl LifecycleEvent {
    fn mount(index: Index) -> Self {
        Self {
            index,
            event_type: LifecycleType::Mount,
        }
    }
}
/// The internal lifecycle event system is managed by these
pub enum LifecycleType {
    Mount,
    PropsChanged,
    Mounted,
    Removed,
    Moved,
    Messaged,
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
    use crate::prelude::*;

    // Make sure this function builds properly.
    fn test_static_fn<'a, P: Properties>(b: &'a Bump, r: FC<P>) -> VNode<'a> {
        todo!()
        // let p = P::new(); // new props
        // let c = Context { props: &p }; // new context with props
        // let g = r(&c); // calling function with context
        // g
    }

    fn test_component<'a>(ctx: &'a Context<()>) -> VNode<'a> {
        // todo: helper should be part of html! macro
        todo!()
        // ctx.view(|bump| html! {bump,  <div> </div> })
    }

    fn test_component2<'a>(ctx: &'a Context<()>) -> VNode<'a> {
        ctx.view(|bump: &Bump| VNode::text("blah"))
    }

    #[test]
    fn ensure_types_work() {
        // TODO: Get the whole casting thing to work properly.
        // For whatever reason, FC is not auto-implemented, depsite it being a static type
        let b = Bump::new();

        // Happiness! The VNodes are now allocated onto the bump vdom
        let nodes0 = test_static_fn(&b, test_component);

        let nodes1 = test_static_fn(&b, test_component2);
    }
}

/// The Scope that wraps a functional component
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components
/// The actualy contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    hook_idx: i32,
    hooks: Vec<OLDHookState>,
    props_type: TypeId,
}

impl Scope {
    // create a new scope from a function
    fn new<T: 'static>(f: FC<T>) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();

        // Obscure the function
        Self {
            hook_idx: 0,
            hooks: vec![],
            props_type,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    fn run() {}
}

pub struct OLDHookState {}

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
pub struct Context<'src, T> {
    /// Direct access to the properties used to create this component.
    pub props: T,
    pub idx: AtomicUsize,
    pub arena: &'src typed_arena::Arena<Hook>,
    pub hooks: RefCell<Vec<*mut Hook>>,
    pub _p: std::marker::PhantomData<&'src ()>,
}

impl<'a, T> Context<'a, T> {
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

    /// use_hook provides a way to store data between renders for functional components.
    pub fn use_hook<'comp, InternalHookState: 'static, Output: 'static>(
        &'comp self,
        // The closure that builds the hook state
        initializer: impl FnOnce() -> InternalHookState,
        // The closure that takes the hookstate and returns some value
        runner: impl for<'b> FnOnce(&'comp mut InternalHookState) -> &'comp Output,
        // The closure that cleans up whatever mess is left when the component gets torn down
        // TODO: add this to the "clean up" group for when the component is dropped
        tear_down: impl FnOnce(InternalHookState),
    ) -> &'comp Output {
        let raw_hook = {
            let idx = self.idx.load(std::sync::atomic::Ordering::Relaxed);

            // Mutate hook list if necessary
            let mut hooks = self.hooks.borrow_mut();

            // Initialize the hook by allocating it in the typed arena.
            // We get a reference from the arena which is owned by the component scope
            // This is valid because "Context" is only valid while the scope is borrowed
            if idx >= hooks.len() {
                let new_state = initializer();
                let boxed_state: Box<dyn std::any::Any> = Box::new(new_state);
                let hook = self.arena.alloc(Hook::new(boxed_state));

                // Push the raw pointer instead of the &mut
                // A "poor man's OwningRef"
                hooks.push(hook);
            }
            self.idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            *hooks.get(idx).unwrap()
        };

        /*
        ** UNSAFETY ALERT **
        Here, we dereference a raw pointer. Normally, we aren't guaranteed that this is okay.

        However, typed-arena gives a mutable reference to the stored data which is stable for any inserts
        into the arena. During the first call of the function, we need to add the mutable reference given to use by
        the arena into our list of hooks. The arena provides stability of the &mut references and is only deallocated
        when the component itself is deallocated.

        This is okay because:
        - The lifetime of the component arena is tied to the lifetime of these raw hooks
        - Usage of the raw hooks is tied behind the Vec refcell
        - Output is static, meaning it can't take a reference to the data
        - We don't expose the raw hook pointer outside of the scope of use_hook
        */
        let borrowed_hook: &'comp mut _ = unsafe { raw_hook.as_mut().unwrap() };

        let internal_state = borrowed_hook
            .state
            .downcast_mut::<InternalHookState>()
            .unwrap();

        runner(internal_state)
    }
}

pub struct Hook {
    state: Box<dyn std::any::Any>,
}

impl Hook {
    fn new(state: Box<dyn std::any::Any>) -> Self {
        Self { state }
    }
}

/// A CallbackEvent wraps any event returned from the renderer's event system.
pub struct CallbackEvent {}

pub struct EventListener {}
