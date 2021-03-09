// use crate::{changelist::EditList, nodes::VNode};

use crate::innerlude::*;
use crate::{
    patch::Edit,
    scope::{create_scoped, Scoped},
};
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
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// eventually, come up with a better datastructure that reuses boxes for known P types
    /// like a generational typemap bump arena
    /// -> IE a cache line for each P type with soem heuristics on optimizing layout
    pub(crate) components: Arena<Box<dyn Scoped>>,
    // pub(crate) components: Rc<RefCell<Arena<Box<dyn Scoped>>>>,
    /// The index of the root component.
    /// Will not be ready if the dom is fresh
    pub(crate) base_scope: ScopeIdx,

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
        // let mut components = Arena::new();
        // let mut components = Arena::new();

        // Create a reference to the component in the arena
        // Note: we are essentially running the "Mount" lifecycle event manually while the vdom doesnt yet exist
        // This puts the dom in a usable state on creation, rather than being potentially invalid
        // let base_scope = components.insert_with(|id| create_scoped(root, root_props, id, None));

        todo!()
        // Self {
        //     // components: RefCell::new(components),
        //     components: components,
        //     // components: Rc::new(RefCell::new(components)),
        //     base_scope,
        //     // event_queue: RefCell::new(VecDeque::new()),
        //     diff_bump: Bump::new(),
        //     _root_prop_type: TypeId::of::<P>(),
        // }
    }

    /// Performs a *full* rebuild of the virtual dom, returning every edit required to generate the actual dom.

    // pub fn rebuild<'s>(&'s mut self) -> Result<> {
    // pub fn rebuild<'s>(&'s mut self) -> Result<std::cell::Ref<'_, Arena<Box<dyn Scoped>>>> {
    pub fn rebuild<'s>(&'s mut self) -> Result<EditList<'s>> {
        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        self.diff_bump.reset();

        self.components
            .get_mut(self.base_scope)
            .expect("Root should always exist")
            .run();

        let b = Bump::new();

        let mut diff_machine = DiffMachine::new(&self.diff_bump);
        // let mut diff_machine = DiffMachine::new(self, &self.diff_bump, self.base_scope);

        // todo!()

        let component = self.components.get(self.base_scope).unwrap();
        diff_machine.diff_node(component.old_frame(), component.new_frame());
        let edits = diff_machine.consume();
        // self.diff_bump = b;
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
        // self.components
        //     .borrow_mut()
        //     .get_mut(event.component_id)
        //     .map(|f| {
        //         f.call_listener(event);
        //         f
        //     })
        //     .map(|f| f.run())
        //     .expect("Borrowing should not fail");

        // component.call_listener(event);

        // .expect("Component should exist if an event was triggered");
        // Reset and then build a new diff machine
        // The previous edit list cannot be around while &mut is held
        // Make sure variance doesnt break this
        // self.diff_bump.reset();
        // let mut diff_machine = DiffMachine::new(&mut self, event.component_id);
        // let mut diff_machine =
        //     DiffMachine::new(&self.diff_bump, &mut self.components, event.component_id);

        // component.run();
        // diff_machine.diff_node(component.old_frame(), component.new_frame());

        todo!()
        // Ok(diff_machine.consume())
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
}

enum LifeCycleEvent {
    // Mount {
//     props: &dyn Properties,
// // f: FC<dyn Properties>,
// },
}
