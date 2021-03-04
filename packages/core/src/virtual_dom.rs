// use crate::{changelist::EditList, nodes::VNode};

use crate::innerlude::*;
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::TypeId,
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
    base_scope: ScopeIdx,

    event_queue: RefCell<VecDeque<LifecycleEvent>>,

    // todo: encapsulate more state into this so we can better reuse it
    diff_bump: Bump,

    // Type of the original props. This is done so VirtualDom does not need to be generic.
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

        let event_queue = RefCell::new(VecDeque::new());

        // Create a reference to the component in the arena
        // Note: we are essentially running the "Mount" lifecycle event manually while the vdom doesnt yet exist
        // This puts the dom in a usable state on creation, rather than being potentially invalid
        let base_scope =
            components.insert_with(|id| Scope::new::<_, P>(root, root_props, id, None));

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

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom.
    ///
    ///
    pub fn rebuild(&mut self) -> Result<EditList<'_>> {
        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        self.diff_bump.reset();
        let mut diff_machine = DiffMachine::new(&self.diff_bump);

        // this is still a WIP
        // we'll need to re-fecth all the scopes that were changed and build the diff machine
        // fetch the component again
        let component = self
            .components
            .get_mut(self.base_scope)
            .expect("Root should always exist");

        component.run::<()>();

        diff_machine.diff_node(component.old_frame(), component.new_frame());

        Ok(diff_machine.consume())
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
    pub fn progress_with_event(&mut self, event: EventTrigger) -> Result<EditList<'_>> {
        let EventTrigger {
            component_id,
            listener_id,
            event: source,
        } = event;

        let component = self
            .components
            .get_mut(component_id)
            .expect("Component should exist if an event was triggered");

        log::debug!("list: {}", component.listeners.len());

        let listener = unsafe {
            component
                .listeners
                .get(listener_id as usize)
                .expect("Listener should exist if it was triggered")
                .as_ref()
        }
        .unwrap();

        // Run the callback with the user event
        listener(source);

        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        self.diff_bump.reset();
        let mut diff_machine = DiffMachine::new(&self.diff_bump);

        // this is still a WIP
        // we'll need to re-fecth all the scopes that were changed and build the diff machine
        // fetch the component again
        // let component = self
        //     .components
        //     .get_mut(self.base_scope)
        //     .expect("Root should always exist");

        component.run::<()>();

        diff_machine.diff_node(component.old_frame(), component.new_frame());
        // diff_machine.diff_node(
        //     component.old_frame(),
        //     component.new_frame(),
        //     Some(self.base_scope),
        // );

        Ok(diff_machine.consume())
        // Err(crate::error::Error::NoEvent)
        // Mark dirty components. Descend from the highest node until all dirty nodes are updated.
        // let mut affected_components = Vec::new();

        // while let Some(event) = self.pop_event() {
        //     if let Some(component_idx) = event.index() {
        //         affected_components.push(component_idx);
        //     }
        //     self.process_lifecycle(event)?;
        // }

        // todo!()
    }

    /// Using mutable access to the Virtual Dom, progress a given lifecycle event
    fn process_lifecycle(&mut self, LifecycleEvent { event_type }: LifecycleEvent) -> Result<()> {
        match event_type {
            // Component needs to be mounted to the virtual dom
            LifecycleType::Mount {
                to: _,
                under: _,
                props: _,
            } => {}

            // The parent for this component generated new props and the component needs update
            LifecycleType::PropsChanged {
                props: _,
                component: _,
            } => {}

            // Component was messaged via the internal subscription service
            LifecycleType::Callback { component: _ } => {}
        }

        Ok(())
    }

    /// Pop the top event of the internal lifecycle event queu
    pub fn pop_event(&self) -> Option<LifecycleEvent> {
        self.event_queue.borrow_mut().pop_front()
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
        to: ScopeIdx,
        under: usize,
        props: Box<dyn std::any::Any>,
    },

    // Parent was evalauted causing new props to generate
    PropsChanged {
        props: Box<dyn std::any::Any>,
        component: ScopeIdx,
    },

    // Hook for the subscription API
    Callback {
        component: ScopeIdx,
    },
}

impl LifecycleEvent {
    fn index(&self) -> Option<ScopeIdx> {
        match &self.event_type {
            LifecycleType::Mount {
                to: _,
                under: _,
                props: _,
            } => None,

            LifecycleType::PropsChanged { component, .. }
            | LifecycleType::Callback { component } => Some(component.clone()),
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn start_dom() {
        let mut dom = VirtualDom::new(|ctx, props| {
            todo!()
            // ctx.render(|ctx| {
            //     use crate::builder::*;
            //     let bump = ctx.bump();
            //     div(bump).child(text("hello,    world")).finish()
            // })
        });
        let edits = dom.rebuild().unwrap();
        println!("{:#?}", edits);
    }
}
