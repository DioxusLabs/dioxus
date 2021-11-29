use bumpalo::Bump;
use futures_channel::mpsc::UnboundedSender;
use fxhash::{FxHashMap, FxHashSet};
use slab::Slab;
use std::{
    borrow::Borrow,
    cell::{Cell, RefCell},
};

use crate::innerlude::*;

pub type FcSlot = *const ();

pub struct Heuristic {
    hook_arena_size: usize,
    node_arena_size: usize,
}

// a slab-like arena with stable references even when new scopes are allocated
// uses a bump arena as a backing
//
// has an internal heuristics engine to pre-allocate arenas to the right size
pub(crate) struct ScopeArena {
    bump: Bump,
    pub pending_futures: RefCell<FxHashSet<ScopeId>>,
    scope_counter: Cell<usize>,
    pub scopes: RefCell<FxHashMap<ScopeId, *mut Scope>>,
    pub heuristics: RefCell<FxHashMap<FcSlot, Heuristic>>,
    free_scopes: RefCell<Vec<*mut Scope>>,
    nodes: RefCell<Slab<*const VNode<'static>>>,
    pub(crate) sender: UnboundedSender<SchedulerMsg>,
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
    pub(crate) fn get_scope(&self, id: &ScopeId) -> Option<&Scope> {
        unsafe { self.scopes.borrow().get(id).map(|f| &**f) }
    }

    pub(crate) unsafe fn get_scope_raw(&self, id: &ScopeId) -> Option<*mut Scope> {
        self.scopes.borrow().get(id).copied()
    }

    pub(crate) unsafe fn get_scope_mut(&self, id: &ScopeId) -> Option<&mut Scope> {
        self.scopes.borrow().get(id).map(|s| &mut **s)
    }

    pub(crate) fn new_with_key(
        &self,
        fc_ptr: *const (),
        caller: *const dyn Fn(&Scope) -> Element,
        parent_scope: Option<*mut Scope>,
        container: ElementId,
        height: u32,
        subtree: u32,
    ) -> ScopeId {
        let new_scope_id = ScopeId(self.scope_counter.get());
        self.scope_counter.set(self.scope_counter.get() + 1);

        // log::debug!("new scope {:?} with parent {:?}", new_scope_id, container);

        if let Some(old_scope) = self.free_scopes.borrow_mut().pop() {
            let scope = unsafe { &mut *old_scope };
            // log::debug!(
            //     "reusing scope {:?} as {:?}",
            //     scope.our_arena_idx,
            //     new_scope_id
            // );

            scope.caller = caller;
            scope.parent_scope = parent_scope;
            scope.height = height;
            scope.subtree = Cell::new(subtree);
            scope.our_arena_idx = new_scope_id;
            scope.container = container;

            scope.frames[0].nodes.get_mut().push({
                let vnode = scope.frames[0]
                    .bump
                    .alloc(VNode::Text(scope.frames[0].bump.alloc(VText {
                        dom_id: Default::default(),
                        is_static: false,
                        text: "",
                    })));
                unsafe { std::mem::transmute(vnode as *mut VNode) }
            });

            scope.frames[1].nodes.get_mut().push({
                let vnode = scope.frames[1]
                    .bump
                    .alloc(VNode::Text(scope.frames[1].bump.alloc(VText {
                        dom_id: Default::default(),
                        is_static: false,
                        text: "",
                    })));
                unsafe { std::mem::transmute(vnode as *mut VNode) }
            });

            let any_item = self.scopes.borrow_mut().insert(new_scope_id, scope);
            debug_assert!(any_item.is_none());
        } else {
            let (node_capacity, hook_capacity) = {
                let heuristics = self.heuristics.borrow();
                if let Some(heuristic) = heuristics.get(&fc_ptr) {
                    (heuristic.node_arena_size, heuristic.hook_arena_size)
                } else {
                    (0, 0)
                }
            };

            let mut frames = [BumpFrame::new(node_capacity), BumpFrame::new(node_capacity)];

            frames[0].nodes.get_mut().push({
                let vnode = frames[0]
                    .bump
                    .alloc(VNode::Text(frames[0].bump.alloc(VText {
                        dom_id: Default::default(),
                        is_static: false,
                        text: "",
                    })));
                unsafe { std::mem::transmute(vnode as *mut VNode) }
            });

            frames[1].nodes.get_mut().push({
                let vnode = frames[1]
                    .bump
                    .alloc(VNode::Text(frames[1].bump.alloc(VText {
                        dom_id: Default::default(),
                        is_static: false,
                        text: "",
                    })));
                unsafe { std::mem::transmute(vnode as *mut VNode) }
            });

            let scope = self.bump.alloc(Scope {
                sender: self.sender.clone(),
                container,
                our_arena_idx: new_scope_id,
                parent_scope,
                height,
                frames,
                subtree: Cell::new(subtree),
                is_subtree_root: Cell::new(false),

                caller,
                generation: 0.into(),

                shared_contexts: Default::default(),

                items: RefCell::new(SelfReferentialItems {
                    listeners: Default::default(),
                    borrowed_props: Default::default(),
                    tasks: Default::default(),
                }),

                hook_arena: Bump::new(),
                hook_vals: RefCell::new(smallvec::SmallVec::with_capacity(hook_capacity)),
                hook_idx: Default::default(),
            });

            let any_item = self.scopes.borrow_mut().insert(new_scope_id, scope);
            debug_assert!(any_item.is_none());
        }

        new_scope_id
    }

    pub fn try_remove(&self, id: &ScopeId) -> Option<()> {
        self.ensure_drop_safety(id);

        // log::debug!("removing scope {:?}", id);

        // Safety:
        // - ensure_drop_safety ensures that no references to this scope are in use
        // - this raw pointer is removed from the map
        let scope = unsafe { &mut *self.scopes.borrow_mut().remove(id).unwrap() };

        // we're just reusing scopes so we need to clear it out
        scope.hook_vals.get_mut().drain(..).for_each(|state| {
            let as_mut = unsafe { &mut *state };
            let boxed = unsafe { bumpalo::boxed::Box::from_raw(as_mut) };
            drop(boxed);
        });
        scope.hook_idx.set(0);
        scope.hook_arena.reset();

        scope.shared_contexts.get_mut().clear();
        scope.parent_scope = None;
        scope.generation.set(0);
        scope.is_subtree_root.set(false);
        scope.subtree.set(0);

        scope.frames[0].nodes.get_mut().clear();
        scope.frames[1].nodes.get_mut().clear();

        scope.frames[0].bump.reset();
        scope.frames[1].bump.reset();

        let SelfReferentialItems {
            borrowed_props,
            listeners,
            tasks,
        } = scope.items.get_mut();

        borrowed_props.clear();
        listeners.clear();
        tasks.clear();

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

    // These methods would normally exist on `scope` but they need access to *all* of the scopes

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
    pub(crate) fn ensure_drop_safety(&self, scope_id: &ScopeId) {
        let scope = self.get_scope(scope_id).unwrap();

        let mut items = scope.items.borrow_mut();

        // make sure we drop all borrowed props manually to guarantee that their drop implementation is called before we
        // run the hooks (which hold an &mut Reference)
        // recursively call ensure_drop_safety on all children
        items.borrowed_props.drain(..).for_each(|comp| {
            let scope_id = comp
                .associated_scope
                .get()
                .expect("VComponents should be associated with a valid Scope");

            self.ensure_drop_safety(&scope_id);

            let mut drop_props = comp.drop_props.borrow_mut().take().unwrap();
            drop_props();
        });

        // Now that all the references are gone, we can safely drop our own references in our listeners.
        items
            .listeners
            .drain(..)
            .for_each(|listener| drop(listener.callback.borrow_mut().take()));
    }

    pub(crate) fn run_scope(&self, id: &ScopeId) -> bool {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(id);

        let scope = unsafe { &mut *self.get_scope_mut(id).expect("could not find scope") };

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        scope.hook_idx.set(0);

        // Safety:
        // - We've dropped all references to the wip bump frame with "ensure_drop_safety"
        unsafe { scope.reset_wip_frame() };

        {
            let mut items = scope.items.borrow_mut();

            // just forget about our suspended nodes while we're at it
            items.tasks.clear();

            // guarantee that we haven't screwed up - there should be no latent references anywhere
            debug_assert!(items.listeners.is_empty());
            debug_assert!(items.borrowed_props.is_empty());
            debug_assert!(items.tasks.is_empty());

            // Todo: see if we can add stronger guarantees around internal bookkeeping and failed component renders.
            scope.wip_frame().nodes.borrow_mut().clear();
        }

        let render: &dyn Fn(&Scope) -> Element = unsafe { &*scope.caller };

        if let Some(link) = render(scope) {
            // right now, it's a panic to render a nodelink from another scope
            // todo: enable this. it should (reasonably) work even if it doesnt make much sense
            assert_eq!(link.scope_id.get(), Some(*id));

            // nodelinks are not assigned when called and must be done so through the create/diff phase
            // however, we need to link this one up since it will never be used in diffing
            scope.wip_frame().assign_nodelink(&link);
            debug_assert_eq!(scope.wip_frame().nodes.borrow().len(), 1);

            if !scope.items.borrow().tasks.is_empty() {
                self.pending_futures.borrow_mut().insert(*id);
            }

            // make the "wip frame" contents the "finished frame"
            // any future dipping into completed nodes after "render" will go through "fin head"
            scope.cycle_frame();
            true
        } else {
            false
        }
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
                            let mut cb = listener.callback.borrow_mut();
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
    pub fn wip_head(&self, id: &ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.wip_frame();
        let nodes = frame.nodes.borrow();
        let node: &VNode = unsafe { &**nodes.get(0).unwrap() };
        unsafe { std::mem::transmute::<&VNode, &VNode>(node) }
    }

    // The head of the bumpframe is the first linked NodeLink
    pub fn fin_head(&self, id: &ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.fin_frame();
        let nodes = frame.nodes.borrow();
        let node: &VNode = unsafe { &**nodes.get(0).unwrap() };
        unsafe { std::mem::transmute::<&VNode, &VNode>(node) }
    }

    pub fn root_node(&self, id: &ScopeId) -> &VNode {
        self.fin_head(id)
    }
}
