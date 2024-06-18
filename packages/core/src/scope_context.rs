use crate::{innerlude::SchedulerMsg, Element, Runtime, ScopeId, Task};
use rustc_hash::FxHashSet;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::Future,
    sync::Arc,
};

/// A component's state separate from its props.
///
/// This struct exists to provide a common interface for all scopes without relying on generics.
pub(crate) struct Scope {
    pub(crate) name: &'static str,
    pub(crate) id: ScopeId,
    pub(crate) parent_id: Option<ScopeId>,
    pub(crate) height: u32,
    pub(crate) render_count: Cell<usize>,

    // Note: the order of the hook and context fields is important. The hooks field must be dropped before the contexts field in case a hook drop implementation tries to access a context.
    pub(crate) hooks: RefCell<Vec<Box<dyn Any>>>,
    pub(crate) hook_index: Cell<usize>,
    pub(crate) shared_contexts: RefCell<Vec<Box<dyn Any>>>,
    pub(crate) spawned_tasks: RefCell<FxHashSet<Task>>,
    /// The task that was last spawned that may suspend. We use this task to check what task to suspend in the event of an early None return from a component
    pub(crate) last_suspendable_task: Cell<Option<Task>>,
    pub(crate) before_render: RefCell<Vec<Box<dyn FnMut()>>>,
    pub(crate) after_render: RefCell<Vec<Box<dyn FnMut()>>>,
}

impl Scope {
    pub(crate) fn new(
        name: &'static str,
        id: ScopeId,
        parent_id: Option<ScopeId>,
        height: u32,
    ) -> Self {
        Self {
            name,
            id,
            parent_id,
            height,
            render_count: Cell::new(0),
            shared_contexts: RefCell::new(vec![]),
            spawned_tasks: RefCell::new(FxHashSet::default()),
            last_suspendable_task: Cell::new(None),
            hooks: RefCell::new(vec![]),
            hook_index: Cell::new(0),
            before_render: RefCell::new(vec![]),
            after_render: RefCell::new(vec![]),
        }
    }

    pub fn parent_id(&self) -> Option<ScopeId> {
        self.parent_id
    }

    fn sender(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        Runtime::with(|rt| rt.sender.clone()).unwrap()
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub fn needs_update(&self) {
        self.needs_update_any(self.id)
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub fn needs_update_any(&self, id: ScopeId) {
        self.sender()
            .unbounded_send(SchedulerMsg::Immediate(id))
            .expect("Scheduler to exist if scope exists");
    }

    /// Create a subscription that schedules a future render for the reference component
    ///
    /// ## Notice: you should prefer using [`Self::schedule_update_any`] and [`Self::scope_id`]
    pub fn schedule_update(&self) -> Arc<dyn Fn() + Send + Sync + 'static> {
        let (chan, id) = (self.sender(), self.id);
        Arc::new(move || drop(chan.unbounded_send(SchedulerMsg::Immediate(id))))
    }

    /// Schedule an update for any component given its [`ScopeId`].
    ///
    /// A component's [`ScopeId`] can be obtained from `use_hook` or the [`current_scope_id`] method.
    ///
    /// This method should be used when you want to schedule an update for a component
    pub fn schedule_update_any(&self) -> Arc<dyn Fn(ScopeId) + Send + Sync> {
        let chan = self.sender();
        Arc::new(move |id| {
            chan.unbounded_send(SchedulerMsg::Immediate(id)).unwrap();
        })
    }

    /// Return any context of type T if it exists on this scope
    pub fn has_context<T: 'static + Clone>(&self) -> Option<T> {
        self.shared_contexts
            .borrow()
            .iter()
            .find_map(|any| any.downcast_ref::<T>())
            .cloned()
    }

    /// Try to retrieve a shared state with type `T` from any parent scope.
    ///
    /// Clones the state if it exists.
    pub fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        tracing::trace!(
            "looking for context {} ({:?}) in {}",
            std::any::type_name::<T>(),
            std::any::TypeId::of::<T>(),
            self.name
        );
        if let Some(this_ctx) = self.has_context() {
            return Some(this_ctx);
        }

        let mut search_parent = self.parent_id;
        let cur_runtime = Runtime::with(|runtime| {
            while let Some(parent_id) = search_parent {
                let parent = runtime.get_state(parent_id).unwrap();
                tracing::trace!(
                    "looking for context {} ({:?}) in {}",
                    std::any::type_name::<T>(),
                    std::any::TypeId::of::<T>(),
                    parent.name
                );
                if let Some(shared) = parent.shared_contexts.borrow().iter().find_map(|any| {
                    tracing::trace!("found context {:?}", (**any).type_id());
                    any.downcast_ref::<T>()
                }) {
                    return Some(shared.clone());
                }
                search_parent = parent.parent_id;
            }
            None
        });

        match cur_runtime.flatten() {
            Some(ctx) => Some(ctx),
            None => {
                tracing::trace!(
                    "context {} ({:?}) not found",
                    std::any::type_name::<T>(),
                    std::any::TypeId::of::<T>()
                );
                None
            }
        }
    }

    /// Inject a Box<dyn Any> into the context of this scope
    pub(crate) fn provide_any_context(&self, mut value: Box<dyn Any>) {
        let mut contexts = self.shared_contexts.borrow_mut();

        // If the context exists, swap it out for the new value
        for ctx in contexts.iter_mut() {
            // Swap the ptr directly
            if ctx.as_ref().type_id() == value.as_ref().type_id() {
                std::mem::swap(ctx, &mut value);
                return;
            }
        }

        // Else, just push it
        contexts.push(value);
    }

    /// Expose state to children further down the [`crate::VirtualDom`] Tree. Requires `Clone` on the context to allow getting values down the tree.
    ///
    /// This is a "fundamental" operation and should only be called during initialization of a hook.
    ///
    /// For a hook that provides the same functionality, use `use_provide_context` and `use_context` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// #[derive(Clone)]
    /// struct SharedState(&'static str);
    ///
    /// // The parent provides context that is available in all children
    /// fn app() -> Element {
    ///     use_hook(|| provide_context(SharedState("world")));
    ///     rsx!(Child {})
    /// }
    ///
    /// // Any child elements can access the context with the `consume_context` function
    /// fn Child() -> Element {
    ///     let state = use_context::<SharedState>();
    ///     rsx!(div { "hello {state.0}" })
    /// }
    /// ```
    pub fn provide_context<T: 'static + Clone>(&self, value: T) -> T {
        tracing::trace!(
            "providing context {} ({:?}) in {}",
            std::any::type_name::<T>(),
            std::any::TypeId::of::<T>(),
            self.name
        );
        let mut contexts = self.shared_contexts.borrow_mut();

        // If the context exists, swap it out for the new value
        for ctx in contexts.iter_mut() {
            // Swap the ptr directly
            if let Some(ctx) = ctx.downcast_mut::<T>() {
                std::mem::swap(ctx, &mut value.clone());
                return value;
            }
        }

        // Else, just push it
        contexts.push(Box::new(value.clone()));

        value
    }

    /// Provide a context to the root and then consume it
    ///
    /// This is intended for "global" state management solutions that would rather be implicit for the entire app.
    /// Things like signal runtimes and routers are examples of "singletons" that would benefit from lazy initialization.
    ///
    /// Note that you should be checking if the context existed before trying to provide a new one. Providing a context
    /// when a context already exists will swap the context out for the new one, which may not be what you want.
    pub fn provide_root_context<T: 'static + Clone>(&self, context: T) -> T {
        Runtime::with(|runtime| {
            runtime
                .get_state(ScopeId::ROOT)
                .unwrap()
                .provide_context(context)
        })
        .expect("Runtime to exist")
    }

    /// Start a new future on the same thread as the rest of the VirtualDom.
    ///
    /// **You should generally use `spawn` instead of this method unless you specifically need to need to run a task during suspense**
    ///
    /// This future will not contribute to suspense resolving but it will run during suspense.
    ///
    /// Because this future runs during suspense, you need to be careful to work with hydration. It is not recommended to do any async IO work in this future, as it can easily cause hydration issues. However, you can use isomorphic tasks to do work that can be consistently replicated on the server and client like logging or responding to state changes.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// // ❌ Do not do requests in isomorphic tasks. It may resolve at a different time on the server and client, causing hydration issues.
    /// let mut state = use_signal(|| None);
    /// spawn_isomorphic(async move {
    ///     state.set(Some(reqwest::get("https://api.example.com").await));
    /// });
    ///
    /// // ✅ You may wait for a signal to change and then log it
    /// let mut state = use_signal(|| 0);
    /// spawn_isomorphic(async move {
    ///     loop {
    ///         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    ///         println!("State is {state}");
    ///     }
    /// });
    /// ```
    pub fn spawn_isomorphic(&self, fut: impl Future<Output = ()> + 'static) -> Task {
        let id = Runtime::with(|rt| rt.spawn_isomorphic(self.id, fut)).expect("Runtime to exist");
        self.spawned_tasks.borrow_mut().insert(id);
        id
    }

    /// Spawns the future and returns the [`Task`]
    pub fn spawn(&self, fut: impl Future<Output = ()> + 'static) -> Task {
        let id = Runtime::with(|rt| rt.spawn(self.id, fut)).expect("Runtime to exist");
        self.spawned_tasks.borrow_mut().insert(id);
        id
    }

    /// Queue an effect to run after the next render
    pub fn queue_effect(&self, f: impl FnOnce() + 'static) {
        Runtime::with(|rt| rt.queue_effect(self.id, f)).expect("Runtime to exist");
    }

    /// Mark this component as suspended on a specific task and then return None
    pub fn suspend(&self, task: Task) -> Option<Element> {
        self.last_suspendable_task.set(Some(task));
        None
    }

    /// Store a value between renders. The foundational hook for all other hooks.
    ///
    /// Accepts an `initializer` closure, which is run on the first use of the hook (typically the initial render).
    /// `use_hook` will return a clone of the value on every render.
    ///
    /// In order to clean up resources you would need to implement the [`Drop`] trait for an inner value stored in a RC or similar (Signals for instance),
    /// as these only drop their inner value once all references have been dropped, which only happens when the component is dropped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dioxus::prelude::*;
    ///
    /// // prints a greeting on the initial render
    /// pub fn use_hello_world() {
    ///     use_hook(|| println!("Hello, world!"));
    /// }
    /// ```
    ///
    /// # Custom Hook Example
    ///
    /// ```rust
    /// use dioxus::prelude::*;
    ///
    /// pub struct InnerCustomState(usize);
    ///
    /// impl Drop for InnerCustomState {
    ///     fn drop(&mut self){
    ///         println!("Component has been dropped.");
    ///     }
    /// }
    ///
    /// #[derive(Clone, Copy)]
    /// pub struct CustomState {
    ///     inner: Signal<InnerCustomState>
    /// }
    ///
    /// pub fn use_custom_state() -> CustomState {
    ///     use_hook(|| CustomState {
    ///         inner: Signal::new(InnerCustomState(0))
    ///     })
    /// }
    /// ```
    pub fn use_hook<State: Clone + 'static>(&self, initializer: impl FnOnce() -> State) -> State {
        let cur_hook = self.hook_index.get();
        let mut hooks = self.hooks.try_borrow_mut().expect("The hook list is already borrowed: This error is likely caused by trying to use a hook inside a hook which violates the rules of hooks.");

        if cur_hook >= hooks.len() {
            hooks.push(Box::new(initializer()));
        }

        hooks
            .get(cur_hook)
            .and_then(|inn| {
                self.hook_index.set(cur_hook + 1);
                let raw_ref: &dyn Any = inn.as_ref();
                raw_ref.downcast_ref::<State>().cloned()
            })
            .expect(
                r#"
                Unable to retrieve the hook that was initialized at this index.
                Consult the `rules of hooks` to understand how to use hooks properly.

                You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
                Functions prefixed with "use" should never be called conditionally.
                "#,
            )
    }

    pub fn push_before_render(&self, f: impl FnMut() + 'static) {
        self.before_render.borrow_mut().push(Box::new(f));
    }

    pub fn push_after_render(&self, f: impl FnMut() + 'static) {
        self.after_render.borrow_mut().push(Box::new(f));
    }

    /// Get the current render since the inception of this component
    ///
    /// This can be used as a helpful diagnostic when debugging hooks/renders, etc
    pub fn generation(&self) -> usize {
        self.render_count.get()
    }

    /// Get the height of this scope
    pub fn height(&self) -> u32 {
        self.height
    }
}

impl ScopeId {
    /// Get the current scope id
    pub fn current_scope_id(self) -> Option<ScopeId> {
        Runtime::with(|rt| rt.current_scope_id()).flatten()
    }

    /// Consume context from the current scope
    pub fn consume_context<T: 'static + Clone>(self) -> Option<T> {
        Runtime::with_scope(self, |cx| cx.consume_context::<T>()).flatten()
    }

    /// Consume context from the current scope
    pub fn consume_context_from_scope<T: 'static + Clone>(self, scope_id: ScopeId) -> Option<T> {
        Runtime::with(|rt| {
            rt.get_state(scope_id)
                .and_then(|cx| cx.consume_context::<T>())
        })
        .flatten()
    }

    /// Check if the current scope has a context
    pub fn has_context<T: 'static + Clone>(self) -> Option<T> {
        Runtime::with_scope(self, |cx| cx.has_context::<T>()).flatten()
    }

    /// Provide context to the current scope
    pub fn provide_context<T: 'static + Clone>(self, value: T) -> T {
        Runtime::with_scope(self, |cx| cx.provide_context(value))
            .expect("to be in a dioxus runtime")
    }

    /// Suspended a component on a specific task and then return None
    pub fn suspend(self, task: Task) -> Option<Element> {
        Runtime::with_scope(self, |cx| {
            cx.suspend(task);
        });
        None
    }

    /// Pushes the future onto the poll queue to be polled after the component renders.
    pub fn push_future(self, fut: impl Future<Output = ()> + 'static) -> Option<Task> {
        Runtime::with_scope(self, |cx| cx.spawn(fut))
    }

    /// Spawns the future but does not return the [`Task`]
    pub fn spawn(self, fut: impl Future<Output = ()> + 'static) {
        Runtime::with_scope(self, |cx| cx.spawn(fut));
    }

    /// Get the current render since the inception of this component
    ///
    /// This can be used as a helpful diagnostic when debugging hooks/renders, etc
    pub fn generation(self) -> Option<usize> {
        Runtime::with_scope(self, |cx| Some(cx.generation())).expect("to be in a dioxus runtime")
    }

    /// Get the parent of the current scope if it exists
    pub fn parent_scope(self) -> Option<ScopeId> {
        Runtime::with_scope(self, |cx| cx.parent_id()).flatten()
    }

    /// Mark the current scope as dirty, causing it to re-render
    pub fn needs_update(self) {
        Runtime::with_scope(self, |cx| cx.needs_update());
    }

    /// Create a subscription that schedules a future render for the reference component. Unlike [`Self::needs_update`], this function will work outside of the dioxus runtime.
    ///
    /// ## Notice: you should prefer using [`schedule_update_any`]
    pub fn schedule_update(&self) -> Arc<dyn Fn() + Send + Sync + 'static> {
        Runtime::with_scope(*self, |cx| cx.schedule_update()).expect("to be in a dioxus runtime")
    }

    /// Get the height of the current scope
    pub fn height(self) -> u32 {
        Runtime::with_scope(self, |cx| cx.height()).expect("to be in a dioxus runtime")
    }

    /// Run a closure inside of scope's runtime
    pub fn in_runtime<T>(self, f: impl FnOnce() -> T) -> T {
        Runtime::current()
            .expect("to be in a dioxus runtime")
            .on_scope(self, f)
    }
}
