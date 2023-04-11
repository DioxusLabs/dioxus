use crate::{
    any_props::AnyProps,
    any_props::VProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    innerlude::{DynamicNode, EventHandler, VComponent, VText},
    innerlude::{ErrorBoundary, Scheduler, SchedulerMsg},
    lazynodes::LazyNodes,
    nodes::{ComponentReturn, IntoAttributeValue, IntoDynNode, RenderReturn},
    AnyValue, Attribute, AttributeValue, Element, Event, Properties, TaskId,
};
use bumpalo::{boxed::Box as BumpBox, Bump};
use bumpslab::{BumpSlab, Slot};
use rustc_hash::{FxHashMap, FxHashSet};
use slab::{Slab, VacantEntry};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug},
    future::Future,
    ops::{Index, IndexMut},
    rc::Rc,
    sync::Arc,
};

/// A wrapper around the [`Scoped`] object that contains a reference to the [`ScopeState`] and properties for a given
/// component.
///
/// The [`Scope`] is your handle to the [`crate::VirtualDom`] and the component state. Every component is given its own
/// [`ScopeState`] and merged with its properties to create a [`Scoped`].
///
/// The [`Scope`] handle specifically exists to provide a stable reference to these items for the lifetime of the
/// component render.
pub type Scope<'a, T = ()> = &'a Scoped<'a, T>;

// This ScopedType exists because we want to limit the amount of monomorphization that occurs when making inner
// state type generic over props. When the state is generic, it causes every method to be monomorphized for every
// instance of Scope<T> in the codebase.
//
//
/// A wrapper around a component's [`ScopeState`] and properties. The [`ScopeState`] provides the majority of methods
/// for the VirtualDom and component state.
pub struct Scoped<'a, T = ()> {
    /// The component's state and handle to the scheduler.
    ///
    /// Stores things like the custom bump arena, spawn functions, hooks, and the scheduler.
    pub scope: &'a ScopeState,

    /// The component's properties.
    pub props: &'a T,
}

impl<'a, T> std::ops::Deref for Scoped<'a, T> {
    type Target = &'a ScopeState;
    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that acts a key for the internal slab of Scopes. This means that the key is not unqiue across
/// time. We do try and guarantee that between calls to `wait_for_work`, no ScopeIds will be recycled in order to give
/// time for any logic that relies on these IDs to properly update.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

/// A thin wrapper around a BumpSlab that uses ids to index into the slab.
pub(crate) struct ScopeSlab {
    slab: BumpSlab<ScopeState>,
    // a slab of slots of stable pointers to the ScopeState in the bump slab
    entries: Slab<Slot<'static, ScopeState>>,
}

impl Drop for ScopeSlab {
    fn drop(&mut self) {
        // Bump slab doesn't drop its contents, so we need to do it manually
        for slot in self.entries.drain() {
            self.slab.remove(slot);
        }
    }
}

impl Default for ScopeSlab {
    fn default() -> Self {
        Self {
            slab: BumpSlab::new(),
            entries: Slab::new(),
        }
    }
}

impl ScopeSlab {
    pub(crate) fn get(&self, id: ScopeId) -> Option<&ScopeState> {
        self.entries.get(id.0).map(|slot| unsafe { &*slot.ptr() })
    }

    pub(crate) fn get_mut(&mut self, id: ScopeId) -> Option<&mut ScopeState> {
        self.entries
            .get(id.0)
            .map(|slot| unsafe { &mut *slot.ptr_mut() })
    }

    pub(crate) fn vacant_entry(&mut self) -> ScopeSlabEntry {
        let entry = self.entries.vacant_entry();
        ScopeSlabEntry {
            slab: &mut self.slab,
            entry,
        }
    }

    pub(crate) fn remove(&mut self, id: ScopeId) {
        self.slab.remove(self.entries.remove(id.0));
    }

    pub(crate) fn contains(&self, id: ScopeId) -> bool {
        self.entries.contains(id.0)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &ScopeState> {
        self.entries.iter().map(|(_, slot)| unsafe { &*slot.ptr() })
    }
}

pub(crate) struct ScopeSlabEntry<'a> {
    slab: &'a mut BumpSlab<ScopeState>,
    entry: VacantEntry<'a, Slot<'static, ScopeState>>,
}

impl<'a> ScopeSlabEntry<'a> {
    pub(crate) fn key(&self) -> ScopeId {
        ScopeId(self.entry.key())
    }

    pub(crate) fn insert(self, scope: ScopeState) -> &'a ScopeState {
        let slot = self.slab.push(scope);
        // this is safe because the slot is only ever accessed with the lifetime of the borrow of the slab
        let slot = unsafe { std::mem::transmute(slot) };
        let entry = self.entry.insert(slot);
        unsafe { &*entry.ptr() }
    }
}

impl Index<ScopeId> for ScopeSlab {
    type Output = ScopeState;
    fn index(&self, id: ScopeId) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl IndexMut<ScopeId> for ScopeSlab {
    fn index_mut(&mut self, id: ScopeId) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

/// A component's state separate from its props.
///
/// This struct exists to provide a common interface for all scopes without relying on generics.
pub struct ScopeState {
    pub(crate) render_cnt: Cell<usize>,
    pub(crate) name: &'static str,

    pub(crate) node_arena_1: BumpFrame,
    pub(crate) node_arena_2: BumpFrame,

    pub(crate) parent: Option<*const ScopeState>,
    pub(crate) id: ScopeId,

    pub(crate) height: u32,

    pub(crate) hook_arena: Bump,
    pub(crate) hook_list: RefCell<Vec<*mut dyn Any>>,
    pub(crate) hook_idx: Cell<usize>,

    pub(crate) shared_contexts: RefCell<FxHashMap<TypeId, Box<dyn Any>>>,

    pub(crate) tasks: Rc<Scheduler>,
    pub(crate) spawned_tasks: RefCell<FxHashSet<TaskId>>,

    pub(crate) borrowed_props: RefCell<Vec<*const VComponent<'static>>>,
    pub(crate) attributes_to_drop: RefCell<Vec<*const Attribute<'static>>>,

    pub(crate) props: Option<Box<dyn AnyProps<'static>>>,
    pub(crate) placeholder: Cell<Option<ElementId>>,
}

impl<'src> ScopeState {
    pub(crate) fn current_frame(&self) -> &BumpFrame {
        match self.render_cnt.get() % 2 {
            0 => &self.node_arena_1,
            1 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }

    pub(crate) fn previous_frame(&self) -> &BumpFrame {
        match self.render_cnt.get() % 2 {
            1 => &self.node_arena_1,
            0 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }

    /// Get the name of this component
    pub fn name(&self) -> &str {
        self.name
    }

    /// Get the current render since the inception of this component
    ///
    /// This can be used as a helpful diagnostic when debugging hooks/renders, etc
    pub fn generation(&self) -> usize {
        self.render_cnt.get()
    }

    /// Get a handle to the currently active bump arena for this Scope
    ///
    /// This is a bump memory allocator. Be careful using this directly since the contents will be wiped on the next render.
    /// It's easy to leak memory here since the drop implementation will not be called for any objects allocated in this arena.
    ///
    /// If you need to allocate items that need to be dropped, use bumpalo's box.
    pub fn bump(&self) -> &Bump {
        // note that this is actually the previous frame since we use that as scratch space while the component is rendering
        self.previous_frame().bump()
    }

    /// Get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Panics if the tree has not been built yet.
    pub fn root_node(&self) -> &RenderReturn {
        self.try_root_node()
            .expect("The tree has not been built yet. Make sure to call rebuild on the tree before accessing its nodes.")
    }

    /// Try to get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Returns [`None`] if the tree has not been built yet.
    pub fn try_root_node(&self) -> Option<&RenderReturn> {
        let ptr = self.current_frame().node.get();

        if ptr.is_null() {
            return None;
        }

        let r: &RenderReturn = unsafe { &*ptr };

        unsafe { std::mem::transmute(r) }
    }

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

    /// Get the Parent of this [`Scope`] within this Dioxus [`crate::VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`crate::VirtualDom`]s or across time. IDs will be reused when components are unmounted.
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
        self.parent.map(|p| unsafe { &*p }.id)
    }

    /// Get the ID of this Scope within this Dioxus [`crate::VirtualDom`].
    ///
    /// This ID is not unique across Dioxus [`crate::VirtualDom`]s or across time. IDs will be reused when components are unmounted.
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
        self.id
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using [`Self::schedule_update_any`] and [`Self::scope_id`]
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
        Arc::new(move |id| {
            chan.unbounded_send(SchedulerMsg::Immediate(id)).unwrap();
        })
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub fn needs_update(&self) {
        self.needs_update_any(self.scope_id());
    }

    /// Get the [`ScopeId`] of a mounted component.
    ///
    /// `ScopeId` is not unique for the lifetime of the [`crate::VirtualDom`] - a [`ScopeId`] will be reused if a component is unmounted.
    pub fn needs_update_any(&self, id: ScopeId) {
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::Immediate(id))
            .expect("Scheduler to exist if scope exists");
    }

    /// Return any context of type T if it exists on this scope
    pub fn has_context<T: 'static + Clone>(&self) -> Option<T> {
        self.shared_contexts
            .borrow()
            .get(&TypeId::of::<T>())?
            .downcast_ref::<T>()
            .cloned()
    }

    /// Try to retrieve a shared state with type `T` from any parent scope.
    ///
    /// Clones the state if it exists.
    pub fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        if let Some(this_ctx) = self.has_context() {
            return Some(this_ctx);
        }

        let mut search_parent = self.parent;
        while let Some(parent_ptr) = search_parent {
            // safety: all parent pointers are valid thanks to the bump arena
            let parent = unsafe { &*parent_ptr };
            if let Some(shared) = parent.shared_contexts.borrow().get(&TypeId::of::<T>()) {
                return shared.downcast_ref::<T>().cloned();
            }
            search_parent = parent.parent;
        }
        None
    }

    /// Expose state to children further down the [`crate::VirtualDom`] Tree. Requires `Clone` on the context to allow getting values down the tree.
    ///
    /// This is a "fundamental" operation and should only be called during initialization of a hook.
    ///
    /// For a hook that provides the same functionality, use `use_provide_context` and `use_context` instead.
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
        let value2 = value.clone();

        self.shared_contexts
            .borrow_mut()
            .insert(TypeId::of::<T>(), Box::new(value));

        value2
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        let id = self.tasks.spawn(self.id, fut);
        self.spawned_tasks.borrow_mut().insert(id);
        id
    }

    /// Spawns the future but does not return the [`TaskId`]
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) {
        self.push_future(fut);
    }

    /// Spawn a future that Dioxus won't clean up when this component is unmounted
    ///
    /// This is good for tasks that need to be run after the component has been dropped.
    pub fn spawn_forever(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        // The root scope will never be unmounted so we can just add the task at the top of the app
        let id = self.tasks.spawn(ScopeId(0), fut);

        // wake up the scheduler if it is sleeping
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::TaskNotified(id))
            .expect("Scheduler should exist");

        self.spawned_tasks.borrow_mut().insert(id);

        id
    }

    /// Informs the scheduler that this task is no longer needed and should be removed.
    ///
    /// This drops the task immediately.
    pub fn remove_future(&self, id: TaskId) {
        self.tasks.remove(id);
    }

    /// Take a lazy [`crate::VNode`] structure and actually build it with the context of the efficient [`bumpalo::Bump`] allocator.
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
    pub fn render(&'src self, rsx: LazyNodes<'src, '_>) -> Element<'src> {
        let element = rsx.call(self);

        let mut listeners = self.attributes_to_drop.borrow_mut();
        for attr in element.dynamic_attrs {
            match attr.value {
                AttributeValue::Any(_) | AttributeValue::Listener(_) => {
                    let unbounded = unsafe { std::mem::transmute(attr as *const Attribute) };
                    listeners.push(unbounded);
                }

                _ => (),
            }
        }

        let mut props = self.borrowed_props.borrow_mut();
        for node in element.dynamic_nodes {
            if let DynamicNode::Component(comp) = node {
                if !comp.static_props {
                    let unbounded = unsafe { std::mem::transmute(comp as *const VComponent) };
                    props.push(unbounded);
                }
            }
        }

        Some(element)
    }

    /// Create a dynamic text node using [`Arguments`] and the [`ScopeState`]'s internal [`Bump`] allocator
    pub fn text_node(&'src self, args: Arguments) -> DynamicNode<'src> {
        DynamicNode::Text(VText {
            value: self.raw_text(args),
            id: Default::default(),
        })
    }

    /// Allocate some text inside the [`ScopeState`] from [`Arguments`]
    ///
    /// Uses the currently active [`Bump`] allocator
    pub fn raw_text(&'src self, args: Arguments) -> &'src str {
        args.as_str().unwrap_or_else(|| {
            use bumpalo::core_alloc::fmt::Write;
            let mut str_buf = bumpalo::collections::String::new_in(self.bump());
            str_buf.write_fmt(args).unwrap();
            str_buf.into_bump_str()
        })
    }

    /// Convert any item that implements [`IntoDynNode`] into a [`DynamicNode`] using the internal [`Bump`] allocator
    pub fn make_node<'c, I>(&'src self, into: impl IntoDynNode<'src, I> + 'c) -> DynamicNode {
        into.into_vnode(self)
    }

    /// Create a new [`Attribute`] from a name, value, namespace, and volatile bool
    ///
    /// "Volatile" referes to whether or not Dioxus should always override the value. This helps prevent the UI in
    /// some renderers stay in sync with the VirtualDom's understanding of the world
    pub fn attr(
        &'src self,
        name: &'static str,
        value: impl IntoAttributeValue<'src>,
        namespace: Option<&'static str>,
        volatile: bool,
    ) -> Attribute<'src> {
        Attribute {
            name,
            namespace,
            volatile,
            mounted_element: Default::default(),
            value: value.into_value(self.bump()),
        }
    }

    /// Create a new [`DynamicNode::Component`] variant
    ///
    ///
    /// The given component can be any of four signatures. Remember that an [`Element`] is really a [`Result<VNode>`].
    ///
    /// ```rust, ignore
    /// // Without explicit props
    /// fn(Scope) -> Element;
    /// async fn(Scope<'_>) -> Element;
    ///
    /// // With explicit props
    /// fn(Scope<Props>) -> Element;
    /// async fn(Scope<Props<'_>>) -> Element;
    /// ```
    pub fn component<P, A, F: ComponentReturn<'src, A>>(
        &'src self,
        component: fn(Scope<'src, P>) -> F,
        props: P,
        fn_name: &'static str,
    ) -> DynamicNode<'src>
    where
        P: Properties + 'src,
    {
        let vcomp = VProps::new(component, P::memoize, props);

        // cast off the lifetime of the render return
        let as_dyn: Box<dyn AnyProps<'src> + '_> = Box::new(vcomp);
        let extended: Box<dyn AnyProps<'src> + 'src> = unsafe { std::mem::transmute(as_dyn) };

        DynamicNode::Component(VComponent {
            name: fn_name,
            render_fn: component as *const (),
            static_props: P::IS_STATIC,
            props: RefCell::new(Some(extended)),
            scope: Cell::new(None),
        })
    }

    /// Create a new [`EventHandler`] from an [`FnMut`]
    pub fn event_handler<T>(&'src self, f: impl FnMut(T) + 'src) -> EventHandler<'src, T> {
        let handler: &mut dyn FnMut(T) = self.bump().alloc(f);
        let caller = unsafe { BumpBox::from_raw(handler as *mut dyn FnMut(T)) };
        let callback = RefCell::new(Some(caller));
        EventHandler { callback }
    }

    /// Create a new [`AttributeValue`] with the listener variant from a callback
    ///
    /// The callback must be confined to the lifetime of the ScopeState
    pub fn listener<T: 'static>(
        &'src self,
        mut callback: impl FnMut(Event<T>) + 'src,
    ) -> AttributeValue<'src> {
        // safety: there's no other way to create a dynamicly-dispatched bump box other than alloc + from-raw
        // This is the suggested way to build a bumpbox
        //
        // In theory, we could just use regular boxes
        let boxed: BumpBox<'src, dyn FnMut(_) + 'src> = unsafe {
            BumpBox::from_raw(self.bump().alloc(move |event: Event<dyn Any>| {
                if let Ok(data) = event.data.downcast::<T>() {
                    callback(Event {
                        propagates: event.propagates,
                        data,
                    });
                }
            }))
        };

        AttributeValue::Listener(RefCell::new(Some(boxed)))
    }

    /// Create a new [`AttributeValue`] with a value that implements [`AnyValue`]
    pub fn any_value<T: AnyValue>(&'src self, value: T) -> AttributeValue<'src> {
        // safety: there's no other way to create a dynamicly-dispatched bump box other than alloc + from-raw
        // This is the suggested way to build a bumpbox
        //
        // In theory, we could just use regular boxes
        let boxed: BumpBox<'src, dyn AnyValue> =
            unsafe { BumpBox::from_raw(self.bump().alloc(value)) };
        AttributeValue::Any(RefCell::new(Some(boxed)))
    }

    /// Inject an error into the nearest error boundary and quit rendering
    ///
    /// The error doesn't need to implement Error or any specific traits since the boundary
    /// itself will downcast the error into a trait object.
    pub fn throw(&self, error: impl Debug + 'static) -> Option<()> {
        if let Some(cx) = self.consume_context::<Rc<ErrorBoundary>>() {
            cx.insert_error(self.scope_id(), Box::new(error));
        }

        // Always return none during a throw
        None
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
        let cur_hook = self.hook_idx.get();
        let mut hook_list = self.hook_list.try_borrow_mut().expect("The hook list is already borrowed: This error is likely caused by trying to use a hook inside a hook which violates the rules of hooks.");

        if cur_hook >= hook_list.len() {
            hook_list.push(self.hook_arena.alloc(initializer()));
        }

        hook_list
            .get(cur_hook)
            .and_then(|inn| {
                self.hook_idx.set(cur_hook + 1);
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
}
