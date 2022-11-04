use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::Arc,
};

use bumpalo::Bump;
use futures_channel::mpsc::UnboundedSender;
use futures_util::Future;

use crate::{
    any_props::AnyProps, arena::ElementId, bump_frame::BumpFrame, future_container::FutureQueue,
    innerlude::SchedulerMsg, lazynodes::LazyNodes, nodes::VNode, suspense::Fiber, TaskId,
};

pub struct Scope<'a, T = ()> {
    pub scope: &'a ScopeState,
    pub props: &'a T,
}

impl<T> Copy for Scope<'_, T> {}

impl<T> Clone for Scope<'_, T> {
    fn clone(&self) -> Self {
        Self {
            props: self.props,
            scope: self.scope,
        }
    }
}

impl<'a, T> std::ops::Deref for Scope<'a, T> {
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

pub struct ScopeState {
    pub render_cnt: usize,

    pub node_arena_1: BumpFrame,
    pub node_arena_2: BumpFrame,

    pub parent: Option<*mut ScopeState>,
    pub container: ElementId,
    pub id: ScopeId,

    pub height: u32,

    pub hook_arena: Bump,
    pub hook_vals: RefCell<Vec<*mut dyn Any>>,
    pub hook_idx: Cell<usize>,

    pub(crate) shared_contexts: RefCell<HashMap<TypeId, Box<dyn Any>>>,

    pub tasks: FutureQueue,

    pub suspense_boundary: Option<Fiber<'static>>,

    pub props: *mut dyn AnyProps<'static>,
}

impl ScopeState {
    pub fn current_arena(&self) -> &BumpFrame {
        match self.render_cnt % 2 {
            0 => &self.node_arena_1,
            1 => &self.node_arena_2,
            _ => unreachable!(),
        }
    }

    pub fn bump(&self) -> &Bump {
        &self.current_arena().bump
    }

    pub fn root_node<'a>(&'a self) -> &'a VNode<'a> {
        let r = unsafe { &*self.current_arena().node.get() };
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

        let mut search_parent = self.parent;

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

            search_parent = parent.parent;
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
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(&self, fut: impl Future<Output = ()> + 'static) -> TaskId {
        // wake up the scheduler if it is sleeping
        self.tasks
            .sender
            .unbounded_send(SchedulerMsg::NewTask(self.id))
            .expect("Scheduler should exist");

        self.tasks.spawn(self.id, fut)
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
            .unbounded_send(SchedulerMsg::NewTask(self.id))
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
}
