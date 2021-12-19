use bumpalo::Bump;
use futures_channel::mpsc::UnboundedSender;
use fxhash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::{
    borrow::Borrow,
    cell::{Cell, RefCell},
};

use crate::innerlude::*;

pub(crate) type FcSlot = *const ();

pub(crate) struct Heuristic {
    hook_arena_size: usize,
    node_arena_size: usize,
}

// a slab-like arena with stable references even when new scopes are allocated
// uses a bump arena as a backing
//
// has an internal heuristics engine to pre-allocate arenas to the right size
pub(crate) struct ScopeArena {
    pub pending_futures: RefCell<FxHashSet<ScopeId>>,
    scope_counter: Cell<usize>,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
    bump: Bump,

    pub scopes: RefCell<FxHashMap<ScopeId, *mut ScopeState>>,
    pub heuristics: RefCell<FxHashMap<FcSlot, Heuristic>>,
    free_scopes: RefCell<Vec<*mut ScopeState>>,
    nodes: RefCell<Slab<*const VNode<'static>>>,
}

impl ScopeArena {
    pub(crate) fn new(sender: UnboundedSender<SchedulerMsg>) -> Self {
        let bump = Bump::new();

        // allocate a container for the root element
        // this will *never* show up in the diffing process
        let el = bump.alloc(VElement {
            tag_name: "root",
            namespace: None,
            key: None,
            dom_id: Cell::new(Some(ElementId(0))),
            parent_id: Default::default(),
            listeners: &[],
            attributes: &[],
            children: &[],
        });

        let node = bump.alloc(VNode::Element(el));
        let mut nodes = Slab::new();
        let root_id = nodes.insert(unsafe { std::mem::transmute(node as *const _) });

        debug_assert_eq!(root_id, 0);

        Self {
            scope_counter: Cell::new(0),
            bump,
            pending_futures: RefCell::new(FxHashSet::default()),
            scopes: RefCell::new(FxHashMap::default()),
            heuristics: RefCell::new(FxHashMap::default()),
            free_scopes: RefCell::new(Vec::new()),
            nodes: RefCell::new(nodes),
            sender,
        }
    }

    /// Safety:
    /// - Obtaining a mutable refernece to any Scope is unsafe
    /// - Scopes use interior mutability when sharing data into components
    pub(crate) fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        unsafe { self.scopes.borrow().get(&id).map(|f| &**f) }
    }

    pub(crate) fn get_scope_raw(&self, id: ScopeId) -> Option<*mut ScopeState> {
        self.scopes.borrow().get(&id).copied()
    }

    pub(crate) fn new_with_key(
        &self,
        fc_ptr: *const (),
        caller: *const dyn Fn(&ScopeState) -> Element,
        parent_scope: Option<*mut ScopeState>,
        container: ElementId,
        height: u32,
        subtree: u32,
    ) -> ScopeId {
        let new_scope_id = ScopeId(self.scope_counter.get());
        self.scope_counter.set(self.scope_counter.get() + 1);

        /*
        This scopearena aggressively reuse old scopes when possible.
        We try to minimize the new allocations for props/arenas.

        However, this will probably lead to some sort of fragmentation.
        I'm not exactly sure how to improve this today.
        */
        match self.free_scopes.borrow_mut().pop() {
            // No free scope, make a new scope
            None => {
                let (node_capacity, hook_capacity) = self
                    .heuristics
                    .borrow()
                    .get(&fc_ptr)
                    .map(|h| (h.node_arena_size, h.hook_arena_size))
                    .unwrap_or_default();

                let frames = [BumpFrame::new(node_capacity), BumpFrame::new(node_capacity)];

                let scope = self.bump.alloc(ScopeState {
                    sender: self.sender.clone(),
                    container,
                    our_arena_idx: new_scope_id,
                    parent_scope,
                    height,
                    frames,
                    subtree: Cell::new(subtree),
                    is_subtree_root: Cell::new(false),

                    caller: Cell::new(caller),
                    generation: 0.into(),

                    shared_contexts: Default::default(),

                    items: RefCell::new(SelfReferentialItems {
                        listeners: Default::default(),
                        borrowed_props: Default::default(),
                        tasks: Default::default(),
                    }),

                    hook_arena: Bump::new(),
                    hook_vals: RefCell::new(Vec::with_capacity(hook_capacity)),
                    hook_idx: Default::default(),
                });

                let any_item = self.scopes.borrow_mut().insert(new_scope_id, scope);
                debug_assert!(any_item.is_none());
            }

            // Reuse a free scope
            Some(old_scope) => {
                let scope = unsafe { &mut *old_scope };
                scope.caller.set(caller);
                scope.parent_scope = parent_scope;
                scope.height = height;
                scope.subtree = Cell::new(subtree);
                scope.our_arena_idx = new_scope_id;
                scope.container = container;
                let any_item = self.scopes.borrow_mut().insert(new_scope_id, scope);
                debug_assert!(any_item.is_none());
            }
        }

        new_scope_id
    }

    // Removes a scope and its descendents from the arena
    pub fn try_remove(&self, id: ScopeId) -> Option<()> {
        self.ensure_drop_safety(id);

        // Safety:
        // - ensure_drop_safety ensures that no references to this scope are in use
        // - this raw pointer is removed from the map
        let scope = unsafe { &mut *self.scopes.borrow_mut().remove(&id).unwrap() };
        scope.reset();

        self.free_scopes.borrow_mut().push(scope);

        Some(())
    }

    pub fn reserve_node(&self, node: &VNode) -> ElementId {
        let mut els = self.nodes.borrow_mut();
        let entry = els.vacant_entry();
        let key = entry.key();
        let id = ElementId(key);
        let node: *const VNode = node as *const _;
        let node = unsafe { std::mem::transmute::<*const VNode, *const VNode>(node) };
        entry.insert(node);
        id
    }

    pub fn update_node(&self, node: &VNode, id: ElementId) {
        let node = unsafe { std::mem::transmute::<*const VNode, *const VNode>(node) };
        *self.nodes.borrow_mut().get_mut(id.0).unwrap() = node;
    }

    pub fn collect_garbage(&self, id: ElementId) {
        self.nodes.borrow_mut().remove(id.0);
    }

    /// This method cleans up any references to data held within our hook list. This prevents mutable aliasing from
    /// causing UB in our tree.
    ///
    /// This works by cleaning up our references from the bottom of the tree to the top. The directed graph of components
    /// essentially forms a dependency tree that we can traverse from the bottom to the top. As we traverse, we remove
    /// any possible references to the data in the hook list.
    ///
    /// References to hook data can only be stored in listeners and component props. During diffing, we make sure to log
    /// all listeners and borrowed props so we can clear them here.
    ///
    /// This also makes sure that drop order is consistent and predictable. All resources that rely on being dropped will
    /// be dropped.
    pub(crate) fn ensure_drop_safety(&self, scope_id: ScopeId) {
        if let Some(scope) = self.get_scope(scope_id) {
            let mut items = scope.items.borrow_mut();

            // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
            // run the hooks (which hold an &mut Reference)
            // recursively call ensure_drop_safety on all children
            items.borrowed_props.drain(..).for_each(|comp| {
                let scope_id = comp
                    .associated_scope
                    .get()
                    .expect("VComponents should be associated with a valid Scope");

                self.ensure_drop_safety(scope_id);

                let mut drop_props = comp.drop_props.borrow_mut().take().unwrap();
                drop_props();
            });

            // Now that all the references are gone, we can safely drop our own references in our listeners.
            items
                .listeners
                .drain(..)
                .for_each(|listener| drop(listener.callback.callback.borrow_mut().take()));
        }
    }

    // pub(crate) fn run_scope(&self, id: ScopeId) -> bool {

    pub(crate) fn run_scope(&self, id: ScopeId) {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(id);

        // todo: we *know* that this is aliased by the contents of the scope itself
        let scope = unsafe { &mut *self.get_scope_raw(id).expect("could not find scope") };

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        scope.hook_idx.set(0);

        // book keeping to ensure safety around the borrowed data
        {
            // Safety:
            // - We've dropped all references to the wip bump frame with "ensure_drop_safety"
            unsafe { scope.reset_wip_frame() };

            let mut items = scope.items.borrow_mut();

            // just forget about our suspended nodes while we're at it
            items.tasks.clear();

            // guarantee that we haven't screwed up - there should be no latent references anywhere
            debug_assert!(items.listeners.is_empty());
            debug_assert!(items.borrowed_props.is_empty());
            debug_assert!(items.tasks.is_empty());
        }

        let render: &dyn Fn(&ScopeState) -> Element = unsafe { &*scope.caller.get() };

        /*
        If the component returns None, then we fill in a placeholder node. This will wipe what was there.

        An alternate approach is to leave the Real Dom the same, but that can lead to safety issues and a lot more checks.
        */
        if let Some(node) = render(scope) {
            if !scope.items.borrow().tasks.is_empty() {
                self.pending_futures.borrow_mut().insert(id);
            }

            let frame = scope.wip_frame();
            let node = frame.bump.alloc(node);
            frame.node.set(unsafe { std::mem::transmute(node) });
        } else {
            let frame = scope.wip_frame();
            let node = frame
                .bump
                .alloc(VNode::Placeholder(frame.bump.alloc(VPlaceholder {
                    dom_id: Default::default(),
                })));
            frame.node.set(unsafe { std::mem::transmute(node) });
        }

        // make the "wip frame" contents the "finished frame"
        // any future dipping into completed nodes after "render" will go through "fin head"
        scope.cycle_frame();
    }

    pub fn call_listener_with_bubbling(&self, event: UserEvent, element: ElementId) {
        let nodes = self.nodes.borrow();
        let mut cur_el = Some(element);

        while let Some(id) = cur_el.take() {
            if let Some(el) = nodes.get(id.0) {
                let real_el = unsafe { &**el };
                if let VNode::Element(real_el) = real_el {
                    //
                    for listener in real_el.listeners.borrow().iter() {
                        if listener.event == event.name {
                            let mut cb = listener.callback.callback.borrow_mut();
                            if let Some(cb) = cb.as_mut() {
                                (cb)(event.data.clone());
                            }
                        }
                    }

                    cur_el = real_el.parent_id.get();
                }
            }
        }
    }

    // The head of the bumpframe is the first linked NodeLink
    pub fn wip_head(&self, id: ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.wip_frame();
        let node = unsafe { &*frame.node.get() };
        unsafe { std::mem::transmute::<&VNode, &VNode>(node) }
    }

    // The head of the bumpframe is the first linked NodeLink
    pub fn fin_head(&self, id: ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.fin_frame();
        let node = unsafe { &*frame.node.get() };
        unsafe { std::mem::transmute::<&VNode, &VNode>(node) }
    }

    pub fn root_node(&self, id: ScopeId) -> &VNode {
        self.fin_head(id)
    }
}

// when dropping the virtualdom, we need to make sure and drop everything important
impl Drop for ScopeArena {
    fn drop(&mut self) {
        for (_, scopeptr) in self.scopes.get_mut().drain() {
            let scope = unsafe { bumpalo::boxed::Box::from_raw(scopeptr) };
            drop(scope);
        }

        // these are probably complete invalid unfortunately ?
        for scopeptr in self.free_scopes.get_mut().drain(..) {
            let scope = unsafe { bumpalo::boxed::Box::from_raw(scopeptr) };
            drop(scope);
        }
    }
}
