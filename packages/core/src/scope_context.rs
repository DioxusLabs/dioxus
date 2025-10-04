use crate::{
    innerlude::{SchedulerMsg, SuspenseContext},
    Runtime, ScopeId, Task,
};
use generational_box::{AnyStorage, Owner};
use rustc_hash::FxHashSet;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    future::Future,
    sync::Arc,
};

pub(crate) enum ScopeStatus {
    Mounted,
    Unmounted {
        // Before the component is mounted, we need to keep track of effects that need to be run once the scope is mounted
        effects_queued: Vec<Box<dyn FnOnce() + 'static>>,
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) enum SuspenseLocation {
    #[default]
    NotSuspended,
    SuspenseBoundary(SuspenseContext),
    UnderSuspense(SuspenseContext),
    InSuspensePlaceholder(SuspenseContext),
}

impl SuspenseLocation {
    pub(crate) fn suspense_context(&self) -> Option<&SuspenseContext> {
        match self {
            SuspenseLocation::InSuspensePlaceholder(context) => Some(context),
            SuspenseLocation::UnderSuspense(context) => Some(context),
            SuspenseLocation::SuspenseBoundary(context) => Some(context),
            _ => None,
        }
    }
}

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
    pub(crate) before_render: RefCell<Vec<Box<dyn FnMut()>>>,
    pub(crate) after_render: RefCell<Vec<Box<dyn FnMut()>>>,

    /// The suspense boundary that this scope is currently in (if any)
    suspense_boundary: SuspenseLocation,

    pub(crate) status: RefCell<ScopeStatus>,
}

impl Scope {
    pub(crate) fn new(
        name: &'static str,
        id: ScopeId,
        parent_id: Option<ScopeId>,
        height: u32,
        suspense_boundary: SuspenseLocation,
    ) -> Self {
        Self {
            name,
            id,
            parent_id,
            height,
            render_count: Cell::new(0),
            shared_contexts: RefCell::new(vec![]),
            spawned_tasks: RefCell::new(FxHashSet::default()),
            hooks: RefCell::new(vec![]),
            hook_index: Cell::new(0),
            before_render: RefCell::new(vec![]),
            after_render: RefCell::new(vec![]),
            status: RefCell::new(ScopeStatus::Unmounted {
                effects_queued: Vec::new(),
            }),
            suspense_boundary,
        }
    }

    pub fn parent_id(&self) -> Option<ScopeId> {
        self.parent_id
    }

    fn sender(&self) -> futures_channel::mpsc::UnboundedSender<SchedulerMsg> {
        Runtime::current().sender.clone()
    }

    /// Mount the scope and queue any pending effects if it is not already mounted
    pub(crate) fn mount(&self, runtime: &Runtime) {
        let mut status = self.status.borrow_mut();
        if let ScopeStatus::Unmounted { effects_queued } = &mut *status {
            for f in effects_queued.drain(..) {
                runtime.queue_effect_on_mounted_scope(self.id, f);
            }
            *status = ScopeStatus::Mounted;
        }
    }

    /// Get the suspense location of this scope
    pub(crate) fn suspense_location(&self) -> SuspenseLocation {
        self.suspense_boundary.clone()
    }

    /// If this scope is a suspense boundary, return the suspense context
    pub(crate) fn suspense_boundary(&self) -> Option<SuspenseContext> {
        match self.suspense_location() {
            SuspenseLocation::SuspenseBoundary(context) => Some(context),
            _ => None,
        }
    }

    /// Check if a node should run during suspense
    pub(crate) fn should_run_during_suspense(&self) -> bool {
        let Some(context) = self.suspense_boundary.suspense_context() else {
            return false;
        };

        !context.frozen()
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub(crate) fn needs_update(&self) {
        self.needs_update_any(self.id)
    }

    /// Mark this scope as dirty, and schedule a render for it.
    pub(crate) fn needs_update_any(&self, id: ScopeId) {
        self.sender()
            .unbounded_send(SchedulerMsg::Immediate(id))
            .expect("Scheduler to exist if scope exists");
    }

    /// Create a subscription that schedules a future render for the referenced component.
    ///
    /// Note: you should prefer using [`Self::schedule_update_any`] and [`Self::id`].
    ///
    /// Note: The function returned by this method will schedule an update for the current component even if it has already updated between when `schedule_update` was called and when the returned function is called.
    /// If the desired behavior is to invalidate the current rendering of the current component (and no-op if already invalidated)
    /// [`subscribe`](crate::reactive_context::ReactiveContext::subscribe) to the [`current`](crate::reactive_context::ReactiveContext::current) [`ReactiveContext`](crate::reactive_context::ReactiveContext) instead.
    pub(crate) fn schedule_update(&self) -> Arc<dyn Fn() + Send + Sync + 'static> {
        let (chan, id) = (self.sender(), self.id);
        Arc::new(move || drop(chan.unbounded_send(SchedulerMsg::Immediate(id))))
    }

    /// Schedule an update for any component given its [`ScopeId`].
    ///
    /// A component's [`ScopeId`] can be obtained from `use_hook` or the [`current_scope_id`](crate::current_scope_id) method.
    ///
    /// This method should be used when you want to schedule an update for a component.
    ///
    /// Note: It does not matter when `schedule_update_any` is called: the returned function will invalidate what ever generation of the specified component is current when returned function is called.
    /// If the desired behavior is to schedule invalidation of the current rendering of a component, use [`ReactiveContext`](crate::reactive_context::ReactiveContext) instead.
    pub(crate) fn schedule_update_any(&self) -> Arc<dyn Fn(ScopeId) + Send + Sync> {
        let chan = self.sender();
        Arc::new(move |id| {
            _ = chan.unbounded_send(SchedulerMsg::Immediate(id));
        })
    }

    /// Get the owner for the current scope.
    pub(crate) fn owner<S: AnyStorage>(&self) -> Owner<S> {
        match self.has_context() {
            Some(rt) => rt,
            None => {
                let owner = S::owner();
                self.provide_context(owner)
            }
        }
    }

    /// Return any context of type T if it exists on this scope
    pub(crate) fn has_context<T: 'static + Clone>(&self) -> Option<T> {
        self.shared_contexts
            .borrow()
            .iter()
            .find_map(|any| any.downcast_ref::<T>())
            .cloned()
    }

    /// Try to retrieve a shared state with type `T` from any parent scope.
    ///
    /// Clones the state if it exists.
    pub(crate) fn consume_context<T: 'static + Clone>(&self) -> Option<T> {
        if let Some(this_ctx) = self.has_context::<T>() {
            return Some(this_ctx);
        }

        let mut search_parent = self.parent_id;

        Runtime::with(|runtime| {
            while let Some(parent_id) = search_parent {
                let parent = runtime.try_get_state(parent_id)?;
                if let Some(shared) = parent.has_context::<T>() {
                    return Some(shared);
                }
                search_parent = parent.parent_id;
            }
            None
        })
    }

    /// Inject a `Box<dyn Any>` into the context of this scope
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
    pub(crate) fn provide_context<T: 'static + Clone>(&self, value: T) -> T {
        let mut contexts = self.shared_contexts.borrow_mut();

        // If the context exists, swap it out for the new value
        for ctx in contexts.iter_mut() {
            // Swap the ptr directly
            if let Some(ctx) = ctx.downcast_mut::<T>() {
                *ctx = value.clone();
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
    pub(crate) fn provide_root_context<T: 'static + Clone>(&self, context: T) -> T {
        Runtime::with(|runtime| runtime.get_state(ScopeId::ROOT).provide_context(context))
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
    /// # use dioxus_core::spawn_isomorphic;
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
    pub(crate) fn spawn_isomorphic(&self, fut: impl Future<Output = ()> + 'static) -> Task {
        let id = Runtime::with(|rt| rt.spawn_isomorphic(self.id, fut));
        self.spawned_tasks.borrow_mut().insert(id);
        id
    }

    /// Spawns the future and returns the [`Task`]
    pub(crate) fn spawn(&self, fut: impl Future<Output = ()> + 'static) -> Task {
        let id = Runtime::with(|rt| rt.spawn(self.id, fut));
        self.spawned_tasks.borrow_mut().insert(id);
        id
    }

    /// Queue an effect to run after the next render
    pub(crate) fn queue_effect(&self, f: impl FnOnce() + 'static) {
        Runtime::with(|rt| rt.queue_effect(self.id, f));
    }

    /// Store a value in the hook list, returning the value.
    pub(crate) fn use_hook<State: Clone + 'static>(
        &self,
        initializer: impl FnOnce() -> State,
    ) -> State {
        let cur_hook = self.hook_index.get();

        // The hook list works by keeping track of the current hook index and pushing the index forward
        // while retrieving the hook value.
        self.hook_index.set(cur_hook + 1);

        let mut hooks = self.hooks
            .try_borrow_mut()
            .expect("The hook list is already borrowed: This error is likely caused by trying to use  hook inside a hook which violates the rules of hooks.");

        // Try and retrieve the hook value if it exists
        if let Some(existing) = self.use_hook_inner::<State>(&mut hooks, cur_hook) {
            return existing;
        }

        // Otherwise, initialize the hook value. In debug mode, we allow hook types to change after a hot patch
        self.push_hook_value(&mut hooks, cur_hook, initializer())
    }

    // The interior version that gets monomorphized by the `State` type but not the `initializer` type.
    // This helps trim down binary sizes
    fn use_hook_inner<State: Clone + 'static>(
        &self,
        hooks: &mut Vec<Box<dyn std::any::Any>>,
        cur_hook: usize,
    ) -> Option<State> {
        hooks.get(cur_hook).and_then(|inn| {
            let raw_ref: &dyn Any = inn.as_ref();
            raw_ref.downcast_ref::<State>().cloned()
        })
    }

    /// Push a new hook value or insert the value into the existing slot, warning if this is not after a hot patch
    fn push_hook_value<State: Clone + 'static>(
        &self,
        hooks: &mut Vec<Box<dyn std::any::Any>>,
        cur_hook: usize,
        value: State,
    ) -> State {
        // If this is a new hook, push it
        if cur_hook >= hooks.len() {
            hooks.push(Box::new(value.clone()));
            return value;
        }

        // If we're in dev mode, we allow swapping hook values if the hook was initialized at this index
        if cfg!(debug_assertions) && unsafe { subsecond::get_jump_table().is_some() } {
            hooks[cur_hook] = Box::new(value.clone());
            return value;
        }

        // Otherwise, panic
        panic!(
            r#"Unable to retrieve the hook that was initialized at this index.
                    Consult the `rules of hooks` to understand how to use hooks properly.

                    You likely used the hook in a conditional. Hooks rely on consistent ordering between renders.
                    Functions prefixed with "use" should never be called conditionally.

                    Help: Run `dx check` to look for check for some common hook errors."#
        );
    }

    pub(crate) fn push_before_render(&self, f: impl FnMut() + 'static) {
        self.before_render.borrow_mut().push(Box::new(f));
    }

    pub(crate) fn push_after_render(&self, f: impl FnMut() + 'static) {
        self.after_render.borrow_mut().push(Box::new(f));
    }

    /// Get the current render since the inception of this component
    ///
    /// This can be used as a helpful diagnostic when debugging hooks/renders, etc
    pub(crate) fn generation(&self) -> usize {
        self.render_count.get()
    }

    /// Get the height of this scope
    pub(crate) fn height(&self) -> u32 {
        self.height
    }
}
