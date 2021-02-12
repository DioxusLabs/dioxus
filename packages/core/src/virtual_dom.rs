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
*/
use crate::inner::*;
use crate::nodes::VNode;
use any::Any;
use bumpalo::Bump;
use generational_arena::{Arena, Index};
use std::{
    any::{self, TypeId},
    cell::{RefCell, UnsafeCell},
    future::Future,
    marker::PhantomData,
    sync::atomic::AtomicUsize,
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom<P: Properties> {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    pub(crate) components: Arena<Scope>,

    /// The index of the root component.
    base_scope: Index,

    /// Components generate lifecycle events
    event_queue: Vec<LifecycleEvent>,

    // mark the root props with P, even though they're held by the root component
    _p: PhantomData<P>,
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
        // 1. Create the component arena
        // 2. Create the base scope (can never be removed)
        // 3. Create the lifecycle queue
        // 4. Create the event queue

        // Arena allocate all the components
        // This should make it *really* easy to store references in events and such
        let mut components = Arena::new();

        // Create a reference to the component in the arena
        let base_scope = components.insert(Scope::new(root, None));

        // Create a new mount event with no root container
        let first_event = LifecycleEvent::mount(base_scope, None, 0, root_props);

        // Create an event queue with a mount for the base scope
        let event_queue = vec![first_event];

        Self {
            components,
            base_scope,
            event_queue,
            _p: PhantomData {},
        }
    }

    /// Pop an event off the event queue and process it
    pub fn progress(&mut self) -> Result<()> {
        let event = self.event_queue.pop().ok_or(Error::NoEvent)?;

        process_event(self, event)
    }

    /// Update the root props, causing a full event cycle
    pub fn update_props(&mut self, new_props: P) {}
}

/// Using mutable access to the Virtual Dom, progress a given lifecycle event
///
///
///
///
///
///
fn process_event<P: Properties>(
    dom: &mut VirtualDom<P>,
    LifecycleEvent { index, event_type }: LifecycleEvent,
) -> Result<()> {
    let scope = dom.components.get(index).ok_or(Error::NoEvent)?;

    match event_type {
        // Component needs to be mounted to the virtual dom
        LifecycleType::Mount { to, under, props } => {
            if let Some(other) = to {
                // mount to another component
            } else {
                // mount to the root
            }

            let g = props.as_ref();
            scope.run(g);
            // scope.run(runner, props, dom);
        }

        // The parent for this component generated new props and the component needs update
        LifecycleType::PropsChanged { props } => {
            //
        }

        // Component was successfully mounted to the dom
        LifecycleType::Mounted {} => {
            //
        }

        // Component was removed from the DOM
        // Run any destructors and cleanup for the hooks and the dump the component
        LifecycleType::Removed {} => {
            let f = dom.components.remove(index);
        }

        // Component was messaged via the internal subscription service
        LifecycleType::Messaged => {
            //
        }

        // Event from renderer was fired with a given listener ID
        //
        LifecycleType::Callback { listener_id } => {}
    }

    Ok(())
}

pub struct LifecycleEvent {
    pub index: Index,
    pub event_type: LifecycleType,
}

/// The internal lifecycle event system is managed by these
/// Right now, we box the properties and but them in the enum
/// Later, we could directly call the chain of children without boxing
/// We could try to reuse the boxes somehow
pub enum LifecycleType {
    Mount {
        to: Option<Index>,
        under: usize,
        props: Box<dyn Properties>,
    },
    PropsChanged {
        props: Box<dyn Properties>,
    },
    Mounted,
    Removed,
    Messaged,
    Callback {
        listener_id: i32,
    },
}

impl LifecycleEvent {
    // helper method for shortcutting to the enum type
    // probably not necessary
    fn mount<P: Properties + 'static>(
        which: Index,
        to: Option<Index>,
        under: usize,
        props: P,
    ) -> Self {
        Self {
            index: which,
            event_type: LifecycleType::Mount {
                to,
                under,
                props: Box::new(props),
            },
        }
    }
}
