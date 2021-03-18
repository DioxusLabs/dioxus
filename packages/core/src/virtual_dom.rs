// use crate::{changelist::EditList, nodes::VNode};

use crate::{error::Error, innerlude::*};
use crate::{patch::Edit, scope::Scope};
use generational_arena::Arena;
use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    rc::{Rc, Weak},
};
use thiserror::private::AsDynError;

// We actually allocate the properties for components in their parent's properties
// We then expose a handle to use those props for render in the form of "OpaqueComponent"
pub(crate) type OpaqueComponent<'a> = dyn for<'b> Fn(Context<'b>) -> DomTree + 'a;

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    components: Arena<Scope>,

    /// The index of the root component.
    /// Will not be ready if the dom is fresh
    base_scope: ScopeIdx,

    // a strong allocation to the "caller" for the original props
    #[doc(hidden)]
    _root_caller: Rc<OpaqueComponent<'static>>,

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
    pub fn new_with_props<P: Properties + 'static>(root: FC<P>, root_props: P) -> Self {
        let mut components = Arena::new();

        // let prr = Rc::new(root_props);

        // the root is kept around with a "hard" allocation
        let root_caller: Rc<OpaqueComponent> = Rc::new(move |ctx| {
            //
            // let p2 = &root_props;
            // let p2 = prr.clone();
            root(ctx, &root_props)
        });

        // we then expose this to the component with a weak allocation
        let weak_caller: Weak<OpaqueComponent> = Rc::downgrade(&root_caller);

        let base_scope = components.insert_with(move |myidx| Scope::new(weak_caller, myidx, None));

        Self {
            components,
            _root_caller: root_caller,
            base_scope,
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    // consume the top of the diff machine event cycle and dump edits into the edit list
    pub fn step(&mut self, event: LifeCycleEvent) -> Result<()> {
        Ok(())
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom.
    pub fn rebuild<'s>(&'s mut self) -> Result<EditList<'s>> {
        // Diff from the top
        let mut diff_machine = DiffMachine::new();

        let very_unsafe_components = &mut self.components as *mut generational_arena::Arena<Scope>;
        let mut component = self
            .components
            .get_mut(self.base_scope)
            .ok_or_else(|| Error::FatalInternal("Acquring base component should never fail"))?;
        component.run_scope()?;
        diff_machine.diff_node(component.old_frame(), component.new_frame());

        // chew down the the lifecycle events until all dirty nodes are computed
        while let Some(event) = diff_machine.lifecycle_events.pop_front() {
            match event {
                // A new component has been computed from the diffing algorithm
                // create a new component in the arena, run it, move the diffing machine to this new spot, and then diff it
                // this will flood the lifecycle queue with new updates
                LifeCycleEvent::Mount { caller, id, scope } => {
                    log::debug!("Mounting a new component");

                    // We're modifying the component arena while holding onto references into the assoiated bump arenas of its children
                    // those references are stable, even if the component arena moves around in memory, thanks to the bump arenas.
                    // However, there is no way to convey this to rust, so we need to use unsafe to pierce through the lifetime.
                    unsafe {
                        let p = &mut *(very_unsafe_components);

                        // todo, hook up the parent/child indexes properly
                        let idx = p.insert_with(|f| Scope::new(caller, f, None));
                        let c = p.get_mut(idx).unwrap();

                        let real_scope = scope.upgrade().unwrap();
                        *real_scope.as_ref().borrow_mut() = Some(idx);
                        c.run_scope()?;
                        diff_machine.change_list.load_known_root(id);
                        diff_machine.diff_node(c.old_frame(), c.new_frame());
                    }
                }
                LifeCycleEvent::PropsChanged { caller, id, scope } => {
                    let idx = scope.upgrade().unwrap().as_ref().borrow().unwrap();
                    unsafe {
                        let p = &mut *(very_unsafe_components);
                        let c = p.get_mut(idx).unwrap();
                        c.update_caller(caller);
                        c.run_scope()?;
                        diff_machine.change_list.load_known_root(id);
                        diff_machine.diff_node(c.old_frame(), c.new_frame());
                    }
                    // break 'render;
                }
                LifeCycleEvent::SameProps { caller, id, scope } => {
                    //
                    // break 'render;
                }
                LifeCycleEvent::Remove => {
                    //
                    // break 'render;
                }
                LifeCycleEvent::Replace { caller, id, .. } => {}
            }

            // } else {
            //     break 'render;
            // }
        }

        let edits: Vec<Edit<'s>> = diff_machine.consume();
        Ok(edits)
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
        let component = self
            .components
            .get_mut(event.component_id)
            .expect("Borrowing should not fail");

        component.call_listener(event);
        component.run_scope()?;

        // let mut diff_machine = DiffMachine::new();
        // let mut diff_machine = DiffMachine::new(&self.diff_bump);

        // diff_machine.diff_node(component.old_frame(), component.new_frame());

        // Ok(diff_machine.consume())
        self.rebuild()
    }
}
