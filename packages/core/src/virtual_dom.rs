// use crate::{changelist::EditList, nodes::VNode};
use crate::{
    changelist::{self, EditList},
    dodriodiff::DiffMachine,
};
use crate::{events::EventTrigger, innerlude::*};

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
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
    pub(crate) components: Arena<Scope>,

    /// The index of the root component.
    /// Will not be ready if the dom is fresh
    base_scope: Index,

    event_queue: Rc<RefCell<VecDeque<LifecycleEvent>>>,

    // todo: encapsulate more state into this so we can better reuse it
    diff_bump: Bump,

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
    pub fn new_with_props<P: 'static>(root: FC<P>, root_props: P) -> Self {
        let mut components = Arena::new();

        let event_queue = Rc::new(RefCell::new(VecDeque::new()));

        // Create a reference to the component in the arena
        // Note: we are essentially running the "Mount" lifecycle event manually while the vdom doesnt yet exist
        // This puts the dom in a usable state on creation, rather than being potentially invalid
        let base_scope = components.insert(Scope::new::<_, P>(root, root_props, None));

        // evaluate the component, pushing any updates its generates into the lifecycle queue
        // todo!

        let _root_prop_type = TypeId::of::<P>();
        let diff_bump = Bump::new();

        Self {
            components,
            base_scope,
            event_queue,
            diff_bump,
            _root_prop_type,
        }
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
    /// Note: this method is not async and does not provide suspense-like functionality. It is up to the renderer to provide the
    /// executor and handlers for suspense as show in the example.
    ///
    /// ```ignore
    /// let (sender, receiver) = channel::new();
    /// sender.send(EventTrigger::start());
    ///
    /// let mut dom = VirtualDom::new();
    /// dom.suspense_handler(|event| sender.send(event));
    ///
    /// while let Ok(diffs) = dom.progress_with_event(receiver.recv().await) {
    ///     render(diffs);
    /// }
    ///
    /// ```
    pub fn progress_with_event(&mut self, evt: EventTrigger) -> Result<EditList<'_>> {
        let EventTrigger {
            component_id,
            listener_id,
            event: _,
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
            if let Some(component_idx) = event.index() {
                affected_components.push(component_idx);
            }
            self.process_lifecycle(event)?;
        }

        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        self.diff_bump.reset();
        let diff_machine = DiffMachine::new(&self.diff_bump);

        Ok(diff_machine.consume())
    }

    /// Using mutable access to the Virtual Dom, progress a given lifecycle event
    fn process_lifecycle(&mut self, LifecycleEvent { event_type }: LifecycleEvent) -> Result<()> {
        match event_type {
            // Component needs to be mounted to the virtual dom
            LifecycleType::Mount { to: _, under: _, props: _ } => {}

            // The parent for this component generated new props and the component needs update
            LifecycleType::PropsChanged { props: _, component: _ } => {}

            // Component was messaged via the internal subscription service
            LifecycleType::Callback { component: _ } => {}
        }

        Ok(())
    }

    /// With access to the virtual dom, schedule an update to the Root component's props.
    /// This generates the appropriate Lifecycle even. It's up to the renderer to actually feed this lifecycle event
    /// back into the event system to get an edit list.
    pub fn update_props<P: 'static>(&mut self, new_props: P) -> Result<LifecycleEvent> {
        // Ensure the props match
        if TypeId::of::<P>() != self._root_prop_type {
            return Err(Error::WrongProps);
        }

        Ok(LifecycleEvent {
            event_type: LifecycleType::PropsChanged {
                props: Box::new(new_props),
                component: self.base_scope,
            },
        })
    }
}

pub struct LifecycleEvent {
    pub event_type: LifecycleType,
}

pub enum LifecycleType {
    // Component needs to be mounted, but its scope doesn't exist yet
    Mount {
        to: Index,
        under: usize,
        props: Box<dyn std::any::Any>,
    },

    // Parent was evalauted causing new props to generate
    PropsChanged {
        props: Box<dyn std::any::Any>,
        component: Index,
    },

    // Hook for the subscription API
    Callback {
        component: Index,
    },
}

impl LifecycleEvent {
    fn index(&self) -> Option<Index> {
        match &self.event_type {
            LifecycleType::Mount { to: _, under: _, props: _ } => None,

            LifecycleType::PropsChanged { component, .. }
            | LifecycleType::Callback { component } => Some(component.clone()),
        }
    }
}
