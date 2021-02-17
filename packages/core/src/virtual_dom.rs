// use crate::{changelist::EditList, nodes::VNode};
use crate::{dodriodiff::DiffMachine, nodes::VNode};
use crate::{events::EventTrigger, innerlude::*};
use any::Any;
use bumpalo::Bump;
use generational_arena::{Arena, Index};
use std::{
    any::{self, TypeId},
    borrow::BorrowMut,
    cell::{RefCell, UnsafeCell},
    collections::{vec_deque, VecDeque},
    future::Future,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::AtomicUsize,
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    // pub struct VirtualDom<P: Properties> {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    pub(crate) components: Arena<Scope>,

    /// The index of the root component.
    base_scope: Index,

    event_queue: Rc<RefCell<VecDeque<LifecycleEvent>>>,

    // Mark the root props with P, even though they're held by the root component
    // This is done so we don't have a "generic" vdom, making it easier to hold references to it, especially when the holders
    // don't care about the generic props type
    // Most implementations that use the VirtualDom won't care about the root props anyways.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,
}

impl VirtualDom {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }

    /// Start a new VirtualDom instance with a dependent props.
    /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
    ///
    /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
    /// to toss out the entire tree.
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
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
        let event_queue = Rc::new(RefCell::new(vec![first_event].into_iter().collect()));

        let _root_prop_type = TypeId::of::<P>();

        Self {
            components,
            base_scope,
            event_queue,
            _root_prop_type,
        }
    }

    /// With access to the virtual dom, schedule an update to the Root component's props
    pub fn update_props<P: Properties + 'static>(&mut self, new_props: P) -> Result<()> {
        // Ensure the props match
        if TypeId::of::<P>() != self._root_prop_type {
            return Err(Error::WrongProps);
        }

        self.event_queue
            .as_ref()
            .borrow_mut()
            .push_back(LifecycleEvent {
                event_type: LifecycleType::PropsChanged {
                    props: Box::new(new_props),
                },
                component_index: self.base_scope,
            });

        Ok(())
    }

    /// Schedule a future update for a component from outside the vdom!
    ///
    /// This lets services external to the virtual dom interact directly with the component and event system.
    pub fn queue_update() {}

    /// Pop an event off the event queue and process it
    /// Update the root props, and progress
    /// Takes a bump arena to allocate into, making the diff phase as fast as possible
    pub fn progress(&mut self) -> Result<()> {
        let event = self
            .event_queue
            .as_ref()
            .borrow_mut()
            .pop_front()
            .ok_or(Error::NoEvent)?;
        self.process_event(event)
    }

    /// This method is the most sophisticated way of updating the virtual dom after an external event has been triggered.
    ///  
    /// Given a synthetic event, the component that triggered the event, and the index of the callback, this runs the virtual
    /// dom to completion, tagging components that need updates, compressing events together, and finally emitting a single
    /// change list.
    ///
    /// If implementing an external renderer, this is the perfect method to combine with an async event loop that waits on
    /// listeners.
    ///
    /// ```ignore
    ///
    ///
    ///
    ///
    /// ```
    pub async fn progress_with_event(&mut self, evt: EventTrigger) -> Result<()> {
        // pub async fn progress_with_event(&mut self, evt: EventTrigger) -> Result<EditList<'_>> {
        let EventTrigger {
            component_id,
            listener_id,
            event,
        } = evt;

        let component = self
            .components
            .get(component_id)
            // todo: update this with a dedicated error type so implementors know what went wrong
            .expect("Component should exist if an event was triggered");

        let listener = component
            .listeners
            .get(listener_id as usize)
            .expect("Listener should exist if it was triggered")
            .as_ref();

        // Run the callback
        // This should cause internal state to progress, dumping events into the event queue
        // todo: integrate this with a tracing mechanism exposed to a dev tool
        listener();

        // Run through our events, tagging which Indexes are receiving updates
        // Prop updates take prescedence over subscription updates
        // Run all prop updates *first* as they will cascade into children.
        // *then* run the non-prop updates that were not already covered by props

        let mut affected_components = Vec::new();
        // It's essentially draining the vec, but with some dancing to release the RefMut
        // We also want to be able to push events into the queue from processing the event
        while let Some(event) = {
            let new_evt = self.event_queue.as_ref().borrow_mut().pop_front();
            new_evt
        } {
            affected_components.push(event.component_index);
            self.process_event(event)?;
        }

        let diff_bump = Bump::new();
        let diff_machine = DiffMachine::new(&diff_bump);

        Ok(())
    }

    pub async fn progress_completely(&mut self) -> Result<()> {
        Ok(())
    }
    /// Using mutable access to the Virtual Dom, progress a given lifecycle event
    ///
    ///
    ///
    ///
    ///
    ///
    fn process_event(
        &mut self,
        LifecycleEvent {
            component_index: index,
            event_type,
        }: LifecycleEvent,
    ) -> Result<()> {
        let scope = self.components.get_mut(index).ok_or(Error::NoEvent)?;

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
                let f = self.components.remove(index);
                // let f = dom.components.remove(index);
            }

            // Component was messaged via the internal subscription service
            LifecycleType::Messaged => {
                //
            }

            // Event from renderer was fired with a given listener ID
            //
            LifecycleType::Callback { listener_id } => {}

            // Run any post-render callbacks on a component
            LifecycleType::Rendered => {}
        }

        Ok(())
    }
}

pub struct LifecycleEvent {
    pub component_index: Index,
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
    Rendered,
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
            component_index: which,
            event_type: LifecycleType::Mount {
                to,
                under,
                props: Box::new(props),
            },
        }
    }
}
