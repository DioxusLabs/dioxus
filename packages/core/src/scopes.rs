use crate::{
    any_props::AnyProps,
    arena::ElementId,
    bump_frame::BumpFrame,
    innerlude::{Scheduler, SchedulerMsg},
    lazynodes::LazyNodes,
    nodes::VNode,
    TaskId,
};
use bumpalo::Bump;
use std::future::Future;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

/// A wrapper around the [`Scoped`] object that contains a reference to the [`ScopeState`] and properties for a given
/// component.
///
/// The [`Scope`] is your handle to the [`VirtualDom`] and the component state. Every component is given its own
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
/// `ScopeId` is a `usize` that is unique across the entire [`VirtualDom`] and across time. [`ScopeID`]s will never be reused
/// once a component has been unmounted.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

/// A component's state.
///
/// This struct stores all the important information about a component's state without the props.
pub struct ScopeState {
    pub(crate) render_cnt: Cell<usize>,

    pub(crate) node_arena_1: BumpFrame,
    pub(crate) node_arena_2: BumpFrame,

    pub(crate) parent: Option<*mut ScopeState>,
    pub(crate) id: ScopeId,

    pub(crate) height: u32,

    pub(crate) hook_arena: Bump,
    pub(crate) hook_list: RefCell<Vec<*mut dyn Any>>,
    pub(crate) hook_idx: Cell<usize>,

    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,

    pub(crate) tasks: Rc<Scheduler>,
    pub(crate) spawned_tasks: HashSet<TaskId>,

    pub(crate) props: *const dyn AnyProps<'static>,
    pub(crate) placeholder: Cell<Option<ElementId>>,
}

impl ScopeState {
    pub fn current_frame(&self) -> &BumpFrame {
        match self.render_cnt.get() % 2 {
            0 => &self.node_arena_1,
            1 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }

    pub fn previous_frame(&self) -> &BumpFrame {
        match self.render_cnt.get() % 2 {
            1 => &self.node_arena_1,
            0 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }

    /// Get a handle to the currently active bump arena for this Scope
    ///
    /// This is a bump memory allocator. Be careful using this directly since the contents will be wiped on the next render.
    /// It's easy to leak memory here since the drop implementation will not be called for any objects allocated in this arena.
    ///
    /// If you need to allocate items that need to be dropped, use bumpalo's box.
    pub fn bump(&self) -> &Bump {
        &self.current_frame().bump
    }

    /// Get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    pub fn root_node<'a>(&'a self) -> &'a VNode<'a> {
        let r = unsafe { &*self.current_frame().node.get() };
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
        self.parent.map(|p| unsafe { &*p }.id)
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
        self.id
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

    /// Mark this scope as dirty, and schedule a render for it.
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

    /// Return any context of type T if it exists on this scope
    pub fn has_context<T: 'static + Clone>(&self) -> Option<T> {
        self.shared_contexts
            .borrow()
            .get(&TypeId::of::<T>())
            .and_then(|shared| shared.downcast_ref::<T>())
            .cloned()
    }

    /// Try to retrieve a shared state with type `T` from any parent scope.
    ///
    /// The state will be cloned and returned, if it exists.
    ///
    /// We recommend wrapping the state in an `Rc` or `Arc` to avoid deep cloning.
    pub fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        if let Some(this_ctx) = self.has_context() {
            return Some(this_ctx);
        }

        let mut search_parent = self.parent;
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
            search_parent = parent.parent;
        }
        None
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

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        self.tasks.spawn(self.id, fut)
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

        id
    }

    /// Informs the scheduler that this task is no longer needed and should be removed.
    ///
    /// This drops the task immediately.
    pub fn remove_future(&self, id: TaskId) {
        self.tasks.remove(id);
    }

    /// Take a lazy [`VNode`] structure and actually build it with the context of the efficient [`Bump`] allocator.
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
        Some(rsx.call(self))
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
        let mut hook_list = self.hook_list.borrow_mut();

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
