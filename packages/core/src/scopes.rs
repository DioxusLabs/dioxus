use crate::{
    dynamic_template_context::TemplateContext,
    innerlude::*,
    template::{
        TemplateAttribute, TemplateElement, TemplateNode, TemplateNodeId, TemplateNodeType,
        TemplateValue, TextTemplateSegment,
    },
    unsafe_utils::extend_vnode,
};
use bumpalo::Bump;
use futures_channel::mpsc::UnboundedSender;
use fxhash::FxHashMap;
use slab::Slab;
use std::{
    any::{Any, TypeId},
    cell::{Cell, Ref, RefCell},
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
};

/// for traceability, we use the raw fn pointer to identify the function
/// we also get the component name, but that's not necessarily unique in the app
pub(crate) type ComponentPtr = *mut std::os::raw::c_void;

pub(crate) struct Heuristic {
    hook_arena_size: usize,
    node_arena_size: usize,
}

// a slab-like arena with stable references even when new scopes are allocated
// uses a bump arena as a backing
//
// has an internal heuristics engine to pre-allocate arenas to the right size
pub(crate) struct ScopeArena {
    pub scope_gen: Cell<usize>,
    pub bump: Bump,
    pub scopes: RefCell<FxHashMap<ScopeId, *mut ScopeState>>,
    pub heuristics: RefCell<FxHashMap<ComponentPtr, Heuristic>>,
    pub free_scopes: RefCell<Vec<*mut ScopeState>>,
    pub nodes: RefCell<Slab<*const VNode<'static>>>,
    pub tasks: Rc<TaskQueue>,
    pub template_resolver: RefCell<TemplateResolver>,
    pub templates: Rc<RefCell<FxHashMap<TemplateId, Rc<RefCell<Template>>>>>,
    // this is used to store intermidiate artifacts of creating templates, so that the lifetime aligns with Mutations<'bump>.
    pub template_bump: Bump,
}

impl ScopeArena {
    pub(crate) fn new(sender: UnboundedSender<SchedulerMsg>) -> Self {
        let bump = Bump::new();

        // allocate a container for the root element
        // this will *never* show up in the diffing process
        // todo: figure out why this is necessary. i forgot. whoops.
        let el = bump.alloc(VElement {
            tag: "root",
            namespace: None,
            key: None,
            id: Cell::new(Some(ElementId(0))),
            parent: Default::default(),
            listeners: &[],
            attributes: &[],
            children: &[],
        });

        let node = bump.alloc(VNode::Element(el));
        let mut nodes = Slab::new();
        let root_id = nodes.insert(unsafe { std::mem::transmute(node as *const _) });

        debug_assert_eq!(root_id, 0);

        Self {
            scope_gen: Cell::new(0),
            bump,
            scopes: RefCell::new(FxHashMap::default()),
            heuristics: RefCell::new(FxHashMap::default()),
            free_scopes: RefCell::new(Vec::new()),
            nodes: RefCell::new(nodes),
            tasks: Rc::new(TaskQueue {
                tasks: RefCell::new(FxHashMap::default()),
                task_map: RefCell::new(FxHashMap::default()),
                gen: Cell::new(0),
                sender,
            }),
            template_resolver: RefCell::new(TemplateResolver::default()),
            templates: Rc::new(RefCell::new(FxHashMap::default())),
            template_bump: Bump::new(),
        }
    }

    /// Safety:
    /// - Obtaining a mutable reference to any Scope is unsafe
    /// - Scopes use interior mutability when sharing data into components
    pub(crate) fn get_scope(&self, id: ScopeId) -> Option<&ScopeState> {
        unsafe { self.scopes.borrow().get(&id).map(|f| &**f) }
    }

    pub(crate) fn get_scope_raw(&self, id: ScopeId) -> Option<*mut ScopeState> {
        self.scopes.borrow().get(&id).copied()
    }

    pub(crate) fn new_with_key(
        &self,
        fc_ptr: ComponentPtr,
        vcomp: Box<dyn AnyProps>,
        parent_scope: Option<ScopeId>,
        container: GlobalNodeId,
    ) -> ScopeId {
        // Increment the ScopeId system. ScopeIDs are never reused
        let new_scope_id = ScopeId(self.scope_gen.get());
        self.scope_gen.set(self.scope_gen.get() + 1);

        // Get the height of the scope
        let height = parent_scope
            .and_then(|id| self.get_scope(id).map(|scope| scope.height + 1))
            .unwrap_or_default();

        let parent_scope = parent_scope.and_then(|f| self.get_scope_raw(f));

        /*
        This scopearena aggressively reuses old scopes when possible.
        We try to minimize the new allocations for props/arenas.

        However, this will probably lead to some sort of fragmentation.
        I'm not exactly sure how to improve this today.
        */
        if let Some(old_scope) = self.free_scopes.borrow_mut().pop() {
            // reuse the old scope
            let scope = unsafe { &mut *old_scope };

            scope.container = container;
            scope.our_arena_idx = new_scope_id;
            scope.parent_scope = parent_scope;
            scope.height = height;
            scope.fnptr = fc_ptr;
            scope.props.get_mut().replace(vcomp);
            scope.frames[0].reset();
            scope.frames[1].reset();
            scope.shared_contexts.get_mut().clear();
            scope.items.get_mut().listeners.clear();
            scope.items.get_mut().borrowed_props.clear();
            scope.hook_idx.set(0);
            scope.hook_vals.get_mut().clear();

            let any_item = self.scopes.borrow_mut().insert(new_scope_id, scope);
            debug_assert!(any_item.is_none());
        } else {
            // else create a new scope
            let (node_capacity, hook_capacity) = self
                .heuristics
                .borrow()
                .get(&fc_ptr)
                .map(|h| (h.node_arena_size, h.hook_arena_size))
                .unwrap_or_default();

            self.scopes.borrow_mut().insert(
                new_scope_id,
                self.bump.alloc(ScopeState {
                    container,
                    our_arena_idx: new_scope_id,
                    parent_scope,
                    height,
                    fnptr: fc_ptr,
                    props: RefCell::new(Some(vcomp)),
                    frames: [BumpFrame::new(node_capacity), BumpFrame::new(node_capacity)],

                    generation: 0.into(),

                    tasks: self.tasks.clone(),
                    shared_contexts: RefCell::default(),

                    items: RefCell::new(SelfReferentialItems {
                        listeners: Vec::default(),
                        borrowed_props: Vec::default(),
                    }),

                    hook_arena: Bump::new(),
                    hook_vals: RefCell::new(Vec::with_capacity(hook_capacity)),
                    hook_idx: Cell::default(),

                    templates: self.templates.clone(),
                }),
            );
        }

        new_scope_id
    }

    // Removes a scope and its descendents from the arena
    pub fn try_remove(&self, id: ScopeId) {
        self.ensure_drop_safety(id);

        // Dispose of any ongoing tasks
        let mut tasks = self.tasks.tasks.borrow_mut();
        let mut task_map = self.tasks.task_map.borrow_mut();
        if let Some(cur_tasks) = task_map.remove(&id) {
            for task in cur_tasks {
                tasks.remove(&task);
            }
        }

        // Safety:
        // - ensure_drop_safety ensures that no references to this scope are in use
        // - this raw pointer is removed from the map
        let scope = unsafe { &mut *self.scopes.borrow_mut().remove(&id).unwrap() };
        scope.reset();

        self.free_scopes.borrow_mut().push(scope);
    }

    pub fn reserve_node<'a>(&self, node: &'a VNode<'a>) -> ElementId {
        let mut els = self.nodes.borrow_mut();
        let entry = els.vacant_entry();
        let key = entry.key();
        let id = ElementId(key);
        let node = unsafe { extend_vnode(node) };
        entry.insert(node as *const _);
        id
    }

    pub fn update_node<'a>(&self, node: &'a VNode<'a>, id: ElementId) {
        let node = unsafe { extend_vnode(node) };
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
                if let Some(scope_id) = comp.scope.get() {
                    self.ensure_drop_safety(scope_id);
                }
                drop(comp.props.take());
            });

            // Now that all the references are gone, we can safely drop our own references in our listeners.
            items
                .listeners
                .drain(..)
                .for_each(|listener| drop(listener.callback.borrow_mut().take()));
        }
    }

    pub(crate) fn run_scope(&self, id: ScopeId) {
        // Cycle to the next frame and then reset it
        // This breaks any latent references, invalidating every pointer referencing into it.
        // Remove all the outdated listeners
        self.ensure_drop_safety(id);

        // todo: we *know* that this is aliased by the contents of the scope itself
        let scope = unsafe { &mut *self.get_scope_raw(id).expect("could not find scope") };

        log::trace!("running scope {:?} symbol: {:?}", id, scope.fnptr);

        // Safety:
        // - We dropped the listeners, so no more &mut T can be used while these are held
        // - All children nodes that rely on &mut T are replaced with a new reference
        scope.hook_idx.set(0);

        {
            // Safety:
            // - We've dropped all references to the wip bump frame with "ensure_drop_safety"
            unsafe { scope.reset_wip_frame() };

            let items = scope.items.borrow();

            // guarantee that we haven't screwed up - there should be no latent references anywhere
            debug_assert!(items.listeners.is_empty());
            debug_assert!(items.borrowed_props.is_empty());
        }

        /*
        If the component returns None, then we fill in a placeholder node. This will wipe what was there.
        An alternate approach is to leave the Real Dom the same, but that can lead to safety issues and a lot more checks.

        Instead, we just treat the `None` as a shortcut to placeholder.
        If the developer wants to prevent a scope from updating, they should control its memoization instead.

        Also, the way we implement hooks allows us to cut rendering short before the next hook is recalled.
        I'm not sure if React lets you abort the component early, but we let you do that.
        */

        let props = scope.props.borrow();
        let render = props.as_ref().unwrap();
        if let Some(node) = render.render(scope) {
            let frame = scope.wip_frame();
            let node = frame.bump.alloc(node);
            frame.node.set(unsafe { extend_vnode(node) });
        } else {
            let frame = scope.wip_frame();
            let node = frame
                .bump
                .alloc(VNode::Placeholder(frame.bump.alloc(VPlaceholder {
                    id: Cell::default(),
                })));
            frame.node.set(unsafe { extend_vnode(node) });
        }

        // make the "wip frame" contents the "finished frame"
        // any future dipping into completed nodes after "render" will go through "fin head"
        scope.cycle_frame();
    }

    pub fn call_listener_with_bubbling(&self, event: &UserEvent, element: GlobalNodeId) {
        let nodes = self.nodes.borrow();
        let mut cur_el = Some(element);

        let state = Rc::new(BubbleState::new());

        while let Some(id) = cur_el.take() {
            if state.canceled.get() {
                // stop bubbling if canceled
                return;
            }
            match id {
                GlobalNodeId::TemplateId {
                    template_ref_id,
                    template_node_id,
                } => {
                    log::trace!(
                        "looking for listener in {:?} in node {:?}",
                        template_ref_id,
                        template_node_id
                    );
                    if let Some(template) = nodes.get(template_ref_id.0) {
                        let template = unsafe { &**template };
                        if let VNode::TemplateRef(template_ref) = template {
                            let templates = self.templates.borrow();
                            let template = templates.get(&template_ref.template_id).unwrap();
                            cur_el = template.borrow().with_node(
                                template_node_id,
                                bubble_template,
                                bubble_template,
                                (
                                    &nodes,
                                    &template_ref.dynamic_context,
                                    &event,
                                    &state,
                                    template_ref_id,
                                ),
                            );
                        }
                    }
                }
                GlobalNodeId::VNodeId(id) => {
                    if let Some(el) = nodes.get(id.0) {
                        let real_el = unsafe { &**el };
                        log::trace!("looking for listener on {:?}", real_el);

                        if let VNode::Element(real_el) = real_el {
                            for listener in real_el.listeners.iter() {
                                if listener.event == event.name {
                                    log::trace!("calling listener {:?}", listener.event);

                                    let mut cb = listener.callback.borrow_mut();
                                    if let Some(cb) = cb.as_mut() {
                                        // todo: arcs are pretty heavy to clone
                                        // we really want to convert arc to rc
                                        // unfortunately, the SchedulerMsg must be send/sync to be sent across threads
                                        // we could convert arc to rc internally or something
                                        (cb)(AnyEvent {
                                            bubble_state: state.clone(),
                                            data: event.data.clone(),
                                        });
                                    }
                                    break;
                                }
                            }

                            cur_el = real_el.parent.get();
                        }
                    }
                }
            }
            if !event.bubbles {
                return;
            }
        }

        fn bubble_template<'b, Attributes, V, Children, Listeners, TextSegments, Text>(
            node: &TemplateNode<Attributes, V, Children, Listeners, TextSegments, Text>,
            ctx: (
                &Ref<Slab<*const VNode>>,
                &TemplateContext<'b>,
                &UserEvent,
                &Rc<BubbleState>,
                ElementId,
            ),
        ) -> Option<GlobalNodeId>
        where
            Attributes: AsRef<[TemplateAttribute<V>]>,
            V: TemplateValue,
            Children: AsRef<[TemplateNodeId]>,
            Listeners: AsRef<[usize]>,
            TextSegments: AsRef<[TextTemplateSegment<Text>]>,
            Text: AsRef<str>,
        {
            let (vnodes, dynamic_context, event, state, template_ref_id) = ctx;
            if let TemplateNodeType::Element(el) = &node.node_type {
                let TemplateElement { listeners, .. } = el;
                for listener_idx in listeners.as_ref() {
                    let listener = dynamic_context.resolve_listener(*listener_idx);
                    if listener.event == event.name {
                        log::trace!("calling listener {:?}", listener.event);

                        let mut cb = listener.callback.borrow_mut();
                        if let Some(cb) = cb.as_mut() {
                            // todo: arcs are pretty heavy to clone
                            // we really want to convert arc to rc
                            // unfortunately, the SchedulerMsg must be send/sync to be sent across threads
                            // we could convert arc to rc internally or something
                            (cb)(AnyEvent {
                                bubble_state: state.clone(),
                                data: event.data.clone(),
                            });
                        }
                        break;
                    }
                }

                if let Some(id) = el.parent {
                    Some(GlobalNodeId::TemplateId {
                        template_ref_id,
                        template_node_id: id,
                    })
                } else {
                    vnodes.get(template_ref_id.0).and_then(|el| {
                        let real_el = unsafe { &**el };
                        if let VNode::Element(real_el) = real_el {
                            real_el.parent.get()
                        } else {
                            None
                        }
                    })
                }
            } else {
                None
            }
        }
    }

    // The head of the bumpframe is the first linked NodeLink
    pub fn wip_head(&self, id: ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.wip_frame();
        let node = unsafe { &*frame.node.get() };
        unsafe { extend_vnode(node) }
    }

    // The head of the bumpframe is the first linked NodeLink
    pub fn fin_head(&self, id: ScopeId) -> &VNode {
        let scope = self.get_scope(id).unwrap();
        let frame = scope.fin_frame();
        let node = unsafe { &*frame.node.get() };
        unsafe { extend_vnode(node) }
    }

    pub fn root_node(&self, id: ScopeId) -> &VNode {
        self.fin_head(id)
    }

    // this is totally okay since all our nodes are always in a valid state
    pub fn get_element(&self, id: ElementId) -> Option<&VNode> {
        self.nodes
            .borrow()
            .get(id.0)
            .copied()
            .map(|ptr| unsafe { extend_vnode(&*ptr) })
    }
}

/// Components in Dioxus use the "Context" object to interact with their lifecycle.
///
/// This lets components access props, schedule updates, integrate hooks, and expose shared state.
///
/// For the most part, the only method you should be using regularly is `render`.
///
/// ## Example
///
/// ```ignore
/// #[derive(Props)]
/// struct ExampleProps {
///     name: String
/// }
///
/// fn Example(cx: Scope<ExampleProps>) -> Element {
///     cx.render(rsx!{ div {"Hello, {cx.props.name}"} })
/// }
/// ```
pub struct Scope<'a, P = ()> {
    /// The internal ScopeState for this component
    pub scope: &'a ScopeState,

    /// The props for this component
    pub props: &'a P,
}

impl<P> Copy for Scope<'_, P> {}
impl<P> Clone for Scope<'_, P> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope,
            props: self.props,
        }
    }
}

impl<'a, P> std::ops::Deref for Scope<'a, P> {
    // rust will auto deref again to the original 'a lifetime at the call site
    type Target = &'a ScopeState;
    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that is unique across the entire [`VirtualDom`] and across time. [`ScopeID`]s will never be reused
/// once a component has been unmounted.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

/// A task's unique identifier.
///
/// `TaskId` is a `usize` that is unique across the entire [`VirtualDom`] and across time. [`TaskID`]s will never be reused
/// once a Task has been completed.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TaskId {
    /// The global ID of the task
    pub id: usize,

    /// The original scope that this task was scheduled in
    pub scope: ScopeId,
}

/// Every component in Dioxus is represented by a `ScopeState`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
///
/// We expose the `Scope` type so downstream users can traverse the Dioxus [`VirtualDom`] for whatever
/// use case they might have.
pub struct ScopeState {
    pub(crate) parent_scope: Option<*mut ScopeState>,
    pub(crate) container: GlobalNodeId,
    pub(crate) our_arena_idx: ScopeId,
    pub(crate) height: u32,
    pub(crate) fnptr: ComponentPtr,
    pub(crate) props: RefCell<Option<Box<dyn AnyProps>>>,

    // nodes, items
    pub(crate) frames: [BumpFrame; 2],
    pub(crate) generation: Cell<u32>,
    pub(crate) items: RefCell<SelfReferentialItems<'static>>,

    // hooks
    pub(crate) hook_arena: Bump,
    pub(crate) hook_vals: RefCell<Vec<*mut dyn Any>>,
    pub(crate) hook_idx: Cell<usize>,

    // shared state -> todo: move this out of scopestate
    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,
    pub(crate) tasks: Rc<TaskQueue>,

    // templates
    pub(crate) templates: Rc<RefCell<FxHashMap<TemplateId, Rc<RefCell<Template>>>>>,
}

pub struct SelfReferentialItems<'a> {
    pub(crate) listeners: Vec<&'a Listener<'a>>,
    pub(crate) borrowed_props: Vec<&'a VComponent<'a>>,
}

// Public methods exposed to libraries and components
impl ScopeState {
    /// Get the height of this Scope - IE the number of scopes above it.
    ///
    /// A Scope with a height of `0` is the root scope - there are no other scopes above it.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.height(), 0);
    /// ```
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the Parent of this [`Scope`] within this Dioxus [`VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`VirtualDom`]s or across time. IDs will be reused when components are unmounted.
    ///
    /// The base component will not have a parent, and will return `None`.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    ///
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.parent(), None);
    /// ```
    pub fn parent(&self) -> Option<ScopeId> {
        // safety: the pointer to our parent is *always* valid thanks to the bump arena
        self.parent_scope.map(|p| unsafe { &*p }.our_arena_idx)
    }

    /// Get the ID of this Scope within this Dioxus [`VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`VirtualDom`]s or across time. IDs will be reused when components are unmounted.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut dom = VirtualDom::new(|cx|  cx.render(rsx!{ div {} }));
    /// dom.rebuild();
    /// let base = dom.base_scope();
    ///
    /// assert_eq!(base.scope_id(), 0);
    /// ```
    pub fn scope_id(&self) -> ScopeId {
        self.our_arena_idx
    }

    /// Get a handle to the raw update scheduler channel
    pub fn scheduler_channel(&self) -> UnboundedSender<SchedulerMsg> {
        self.tasks.sender.clone()
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using [`schedule_update_any`] and [`scope_id`]
    pub fn schedule_update(&self) -> Arc<dyn Fn() + Send + Sync + 'static> {
        let (chan, id) = (self.tasks.sender.clone(), self.scope_id());
        Arc::new(move || drop(chan.unbounded_send(SchedulerMsg::Immediate(id))))
    }

    /// Schedule an update for any component given its [`ScopeId`].
    ///
    /// A component's [`ScopeId`] can be obtained from `use_hook` or the [`ScopeState::scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Arc<dyn Fn(ScopeId) + Send + Sync> {
        let chan = self.tasks.sender.clone();
        Arc::new(move |id| drop(chan.unbounded_send(SchedulerMsg::Immediate(id))))
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the [`VirtualDom`] - a [`ScopeId`] will be reused if a component is unmounted.
    pub fn needs_update(&self) {
        self.needs_update_any(self.scope_id());
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the [`VirtualDom`] - a [`ScopeId`] will be reused if a component is unmounted.
    pub fn needs_update_any(&self, id: ScopeId) {
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::Immediate(id))
            .expect("Scheduler to exist if scope exists");
    }

    /// Get the Root Node of this scope
    pub fn root_node(&self) -> &VNode {
        let node = unsafe { &*self.fin_frame().node.get() };
        unsafe { std::mem::transmute(node) }
    }

    /// This method enables the ability to expose state to children further down the [`VirtualDom`] Tree.
    ///
    /// This is a "fundamental" operation and should only be called during initialization of a hook.
    ///
    /// For a hook that provides the same functionality, use `use_provide_context` and `use_consume_context` instead.
    ///
    /// When the component is dropped, so is the context. Be aware of this behavior when consuming
    /// the context via Rc/Weak.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// struct SharedState(&'static str);
    ///
    /// static App: Component = |cx| {
    ///     cx.use_hook(|| cx.provide_context(SharedState("world")));
    ///     render!(Child {})
    /// }
    ///
    /// static Child: Component = |cx| {
    ///     let state = cx.consume_state::<SharedState>();
    ///     render!(div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_context<T: 'static + Clone>(&self, value: T) -> T {
        self.shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Box::new(value.clone()))
            .and_then(|f| f.downcast::<T>().ok());
        value
    }

    /// Provide a context for the root component from anywhere in your app.
    ///
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// struct SharedState(&'static str);
    ///
    /// static App: Component = |cx| {
    ///     cx.use_hook(|| cx.provide_root_context(SharedState("world")));
    ///     render!(Child {})
    /// }
    ///
    /// static Child: Component = |cx| {
    ///     let state = cx.consume_state::<SharedState>();
    ///     render!(div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_root_context<T: 'static + Clone>(&self, value: T) -> T {
        // if we *are* the root component, then we can just provide the context directly
        if self.scope_id() == ScopeId(0) {
            self.shared_contexts
                .borrow_mut()
                .insert(TypeId::of::<T>(), Box::new(value.clone()))
                .and_then(|f| f.downcast::<T>().ok());
            return value;
        }

        let mut search_parent = self.parent_scope;

        while let Some(parent) = search_parent.take() {
            let parent = unsafe { &*parent };

            if parent.scope_id() == ScopeId(0) {
                let exists = parent
                    .shared_contexts
                    .borrow_mut()
                    .insert(TypeId::of::<T>(), Box::new(value.clone()));

                if exists.is_some() {
                    log::warn!("Context already provided to parent scope - replacing it");
                }
                return value;
            }

            search_parent = parent.parent_scope;
        }

        unreachable!("all apps have a root scope")
    }

    /// Try to retrieve a shared state with type T from the any parent Scope.
    pub fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        if let Some(shared) = self.shared_contexts.borrow().get(&TypeId::of::<T>()) {
            Some(
                (*shared
                    .downcast_ref::<T>()
                    .expect("Context of type T should exist"))
                .clone(),
            )
        } else {
            let mut search_parent = self.parent_scope;

            while let Some(parent_ptr) = search_parent {
                // safety: all parent pointers are valid thanks to the bump arena
                let parent = unsafe { &*parent_ptr };
                if let Some(shared) = parent.shared_contexts.borrow().get(&TypeId::of::<T>()) {
                    return Some(
                        shared
                            .downcast_ref::<T>()
                            .expect("Context of type T should exist")
                            .clone(),
                    );
                }
                search_parent = parent.parent_scope;
            }
            None
        }
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        // wake up the scheduler if it is sleeping
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::NewTask(self.our_arena_idx))
            .expect("Scheduler should exist");

        self.tasks.spawn(self.our_arena_idx, fut)
    }

    /// Spawns the future but does not return the [`TaskId`]
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.push_future(fut);
    }

    /// Spawn a future that Dioxus will never clean up
    ///
    /// This is good for tasks that need to be run after the component has been dropped.
    pub fn spawn_forever(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        // wake up the scheduler if it is sleeping
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::NewTask(self.our_arena_idx))
            .expect("Scheduler should exist");

        // The root scope will never be unmounted so we can just add the task at the top of the app
        self.tasks.spawn(ScopeId(0), fut)
    }

    /// Informs the scheduler that this task is no longer needed and should be removed
    /// on next poll.
    pub fn remove_future(&self, id: TaskId) {
        self.tasks.remove(id);
    }

    /// Take a lazy [`VNode`] structure and actually build it with the context of the Vdoms efficient [`VNode`] allocator.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// fn Component(cx: Scope<Props>) -> Element {
    ///     // Lazy assemble the VNode tree
    ///     let lazy_nodes = rsx!("hello world");
    ///
    ///     // Actually build the tree and allocate it
    ///     cx.render(lazy_tree)
    /// }
    ///```
    pub fn render<'src>(&'src self, rsx: LazyNodes<'src, '_>) -> Option<VNode<'src>> {
        Some(rsx.call(NodeFactory {
            scope: self,
            bump: &self.wip_frame().bump,
        }))
    }

    /// Store a value between renders. The foundational hook for all other hooks.
    ///
    /// Accepts an `initializer` closure, which is run on the first use of the hook (typically the initial render). The return value of this closure is stored for the lifetime of the component, and a mutable reference to it is provided on every render as the return value of `use_hook`.
    ///
    /// When the component is unmounted (removed from the UI), the value is dropped. This means you can return a custom type and provide cleanup code by implementing the [`Drop`] trait
    ///
    /// # Example
    ///
    /// ```
    /// use dioxus_core::ScopeState;
    ///
    /// // prints a greeting on the initial render
    /// pub fn use_hello_world(cx: &ScopeState) {
    ///     cx.use_hook(|| println!("Hello, world!"));
    /// }
    /// ```
    #[allow(clippy::mut_from_ref)]
    pub fn use_hook<State: 'static>(&self, initializer: impl FnOnce() -> State) -> &mut State {
        let mut vals = self.hook_vals.borrow_mut();

        let hook_len = vals.len();
        let cur_idx = self.hook_idx.get();

        if cur_idx >= hook_len {
            vals.push(self.hook_arena.alloc(initializer()));
        }

        vals
            .get(cur_idx)
            .and_then(|inn| {
                self.hook_idx.set(cur_idx + 1);
                let raw_box = unsafe { &mut **inn };
                raw_box.downcast_mut::<State>()
            })
            .expect(
                r###"
                Unable to retrieve the hook that was initialized at this index.
                Consult the `rules of hooks` to understand how to use hooks properly.

                You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
                Functions prefixed with "use" should never be called conditionally.
                "###,
            )
    }

    /// The "work in progress frame" represents the frame that is currently being worked on.
    pub(crate) fn wip_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 {
            0 => &self.frames[0],
            _ => &self.frames[1],
        }
    }

    /// Mutable access to the "work in progress frame" - used to clear it
    pub(crate) fn wip_frame_mut(&mut self) -> &mut BumpFrame {
        match self.generation.get() & 1 {
            0 => &mut self.frames[0],
            _ => &mut self.frames[1],
        }
    }

    /// Access to the frame where finalized nodes existed
    pub(crate) fn fin_frame(&self) -> &BumpFrame {
        match self.generation.get() & 1 {
            1 => &self.frames[0],
            _ => &self.frames[1],
        }
    }

    /// Reset this component's frame
    ///
    /// # Safety:
    ///
    /// This method breaks every reference of every [`VNode`] in the current frame.
    ///
    /// Calling reset itself is not usually a big deal, but we consider it important
    /// due to the complex safety guarantees we need to uphold.
    pub(crate) unsafe fn reset_wip_frame(&mut self) {
        self.wip_frame_mut().bump.reset();
    }

    /// Cycle to the next generation
    pub(crate) fn cycle_frame(&self) {
        self.generation.set(self.generation.get() + 1);
    }

    // todo: disable bookkeeping on drop (unncessary)
    pub(crate) fn reset(&mut self) {
        // first: book keaping
        self.hook_idx.set(0);
        self.parent_scope = None;
        self.generation.set(0);

        // next: shared context data
        self.shared_contexts.get_mut().clear();

        // next: reset the node data
        let SelfReferentialItems {
            borrowed_props,
            listeners,
        } = self.items.get_mut();
        borrowed_props.clear();
        listeners.clear();
        self.frames[0].reset();
        self.frames[1].reset();

        // Free up the hook values
        self.hook_vals.get_mut().drain(..).for_each(|state| {
            let as_mut = unsafe { &mut *state };
            let boxed = unsafe { bumpalo::boxed::Box::from_raw(as_mut) };
            drop(boxed);
        });

        // Finally, clear the hook arena
        self.hook_arena.reset();
    }
}

pub(crate) struct BumpFrame {
    pub bump: Bump,
    pub node: Cell<*const VNode<'static>>,
}
impl BumpFrame {
    pub(crate) fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        let node = bump.alloc(VText {
            text: "placeholdertext",
            id: Cell::default(),
            is_static: false,
        });
        let node = bump.alloc(VNode::Text(unsafe {
            &*(node as *mut VText as *const VText)
        }));
        let nodes = Cell::new(node as *const _);
        Self { bump, node: nodes }
    }

    pub(crate) fn reset(&mut self) {
        self.bump.reset();
        let node = self.bump.alloc(VText {
            text: "placeholdertext",
            id: Cell::default(),
            is_static: false,
        });
        let node = self.bump.alloc(VNode::Text(unsafe {
            &*(node as *mut VText as *const VText)
        }));
        self.node.set(node as *const _);
    }
}

pub(crate) struct TaskQueue {
    pub(crate) tasks: RefCell<FxHashMap<TaskId, InnerTask>>,
    pub(crate) task_map: RefCell<FxHashMap<ScopeId, HashSet<TaskId>>>,
    gen: Cell<usize>,
    sender: UnboundedSender<SchedulerMsg>,
}

pub(crate) type InnerTask = Pin<Box<dyn Future<Output = ()>>>;
impl TaskQueue {
    fn spawn(&self, scope: ScopeId, task: impl Future<Output = ()> + 'static) -> TaskId {
        let pinned = Box::pin(task);
        let id = self.gen.get();
        self.gen.set(id + 1);
        let tid = TaskId { id, scope };

        self.tasks.borrow_mut().insert(tid, pinned);

        // also add to the task map
        // when the component is unmounted we know to remove it from the map
        self.task_map
            .borrow_mut()
            .entry(scope)
            .or_default()
            .insert(tid);

        tid
    }

    fn remove(&self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.try_borrow_mut() {
            tasks.remove(&id);
            if let Some(task_map) = self.task_map.borrow_mut().get_mut(&id.scope) {
                task_map.remove(&id);
            }
        }
        // the task map is still around, but it'll be removed when the scope is unmounted
    }

    pub(crate) fn has_tasks(&self) -> bool {
        !self.tasks.borrow().is_empty()
    }
}

#[test]
fn sizeof() {
    dbg!(std::mem::size_of::<ScopeState>());
}
