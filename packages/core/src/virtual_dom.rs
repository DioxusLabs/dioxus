// use crate::{changelist::EditList, nodes::VNode};

use crate::innerlude::*;
use crate::{patch::Edit, scope::Scope};
use bumpalo::Bump;
use generational_arena::Arena;
use std::{
    any::TypeId,
    cell::RefCell,
    rc::{Rc, Weak},
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// eventually, come up with a better datastructure that reuses boxes for known P types
    /// like a generational typemap bump arena
    /// -> IE a cache line for each P type with some heuristics on optimizing layout
    pub(crate) components: Arena<Scope>,
    // pub(crate) components: RefCell<Arena<Box<dyn Scoped>>>,
    // pub(crate) components: Rc<RefCell<Arena<Box<dyn Scoped>>>>,
    /// The index of the root component.
    /// Will not be ready if the dom is fresh
    pub(crate) base_scope: ScopeIdx,

    pub(crate) root_caller: Rc<dyn Fn(Context) -> DomTree + 'static>,

    // Type of the original props. This is done so VirtualDom does not need to be generic.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,
    // ======================
    //  DIFF RELATED ITEMs
    // ======================
    // // todo: encapsulate more state into this so we can better reuse it
    pub(crate) diff_bump: Bump,
    // // be very very very very very careful
    // pub change_list: EditMachine<'static>,

    // // vdom: &'a VirtualDom,
    // vdom: *mut Arena<Box<dyn Scoped>>,

    // // vdom: Rc<RefCell<Arena<Box<dyn Scoped>>>>,
    // pub cur_idx: ScopeIdx,

    // // todo
    // // do an indexmap sorted by height
    // dirty_nodes: fxhash::FxHashSet<ScopeIdx>,
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

        // the root is kept around
        let root_caller: Rc<dyn Fn(Context) -> DomTree + 'static> =
            Rc::new(move |ctx| root(ctx, &root_props));
        let weak_caller: Weak<dyn Fn(Context) -> DomTree + 'static> = Rc::downgrade(&root_caller);
        let base_scope = components.insert_with(move |id| Scope::new(weak_caller, id, None));

        Self {
            components,
            root_caller,
            base_scope,
            diff_bump: Bump::new(),
            _root_prop_type: TypeId::of::<P>(),
        }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom.
    pub fn rebuild<'s>(&'s mut self) -> Result<EditList<'s>> {
        log::debug!("rebuilding...");
        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        // bump.reset();

        // Diff from the top
        let mut diff_machine = DiffMachine::new(); // partial borrow
        {
            let component = self
                .components
                .get_mut(self.base_scope)
                .expect("failed to acquire base scope");

            component.run();
        }

        {
            let component = self
                .components
                .get(self.base_scope)
                .expect("failed to acquire base scope");

            diff_machine.diff_node(component.old_frame(), component.new_frame());
        }

        // 'render: loop {
        //     for event in &mut diff_machine.lifecycle_events.drain(..) {
        //         log::debug!("event is {:#?}", event);
        //         match event {
        //             LifeCycleEvent::Mount { caller, id } => {
        //                 diff_machine.change_list.load_known_root(id);
        //                 let idx = self
        //                     .components
        //                     .insert_with(|f| create_scoped(caller, f, None));
        //                 // .insert_with(|f| create_scoped(caller, props, myidx, parent));
        //             }
        //             LifeCycleEvent::PropsChanged => {
        //                 //
        //                 break 'render;
        //             }
        //             LifeCycleEvent::SameProps => {
        //                 //
        //                 break 'render;
        //             }
        //             LifeCycleEvent::Remove => {
        //                 //
        //                 break 'render;
        //             }
        //         }
        //     }

        //     if diff_machine.lifecycle_events.is_empty() {
        //         break 'render;
        //     }
        // }

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
        component.run();

        let mut diff_machine = DiffMachine::new();
        // let mut diff_machine = DiffMachine::new(&self.diff_bump);

        diff_machine.diff_node(component.old_frame(), component.new_frame());

        Ok(diff_machine.consume())
    }
}
