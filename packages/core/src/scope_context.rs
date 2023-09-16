use crate::{
    innerlude::{ErrorBoundary, Scheduler, SchedulerMsg},
    runtime::{with_current_scope, with_runtime},
    Element, ScopeId, TaskId,
};
use rustc_hash::FxHashSet;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt::Debug,
    future::Future,
    rc::Rc,
    sync::Arc,
};

/// A component's state separate from its props.
///
/// This struct exists to provide a common interface for all scopes without relying on generics.
pub(crate) struct ScopeContext {
    pub(crate) name: &'static str,

    pub(crate) id: ScopeId,
    pub(crate) parent_id: Option<ScopeId>,

    pub(crate) height: u32,
    pub(crate) suspended: Cell<bool>,

    pub(crate) shared_contexts: RefCell<Vec<Box<dyn Any>>>,

    pub(crate) tasks: Rc<Scheduler>,
    pub(crate) spawned_tasks: RefCell<FxHashSet<TaskId>>,
}

impl ScopeContext {
    pub(crate) fn new(
        name: &'static str,
        id: ScopeId,
        parent_id: Option<ScopeId>,
        height: u32,
        tasks: Rc<Scheduler>,
    ) -> Self {
        Self {
            name,
            id,
            parent_id,
            height,
            suspended: Cell::new(false),
            shared_contexts: RefCell::new(vec![]),
            tasks,
            spawned_tasks: RefCell::new(FxHashSet::default()),
        }
    }

    pub fn parent_id(&self) -> Option<ScopeId> {
        self.parent_id
    }

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
        match with_runtime(|runtime: &crate::runtime::Runtime| {
            while let Some(parent_id) = search_parent {
                let parent = runtime.get_context(parent_id).unwrap();
                tracing::trace!(
                    "looking for context {} ({:?}) in {}",
                    std::any::type_name::<T>(),
                    std::any::TypeId::of::<T>(),
                    parent.name
                );
                if let Some(shared) = parent.shared_contexts.borrow().iter().find_map(|any| {
                    tracing::trace!("found context {:?}", any.type_id());
                    any.downcast_ref::<T>()
                }) {
                    return Some(shared.clone());
                }
                search_parent = parent.parent_id;
            }
            None
        })
        .flatten()
        {
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
        with_runtime(|runtime| {
            runtime
                .get_context(ScopeId::ROOT)
                .unwrap()
                .provide_context(context)
        })
        .expect("Runtime to exist")
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
        let id = self.tasks.spawn(ScopeId::ROOT, fut);

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

    /// Mark this component as suspended and then return None
    pub fn suspend(&self) -> Option<Element> {
        self.suspended.set(true);
        None
    }
}

/// Schedule an update for any component given its [`ScopeId`].
///
/// A component's [`ScopeId`] can be obtained from `use_hook` or the [`crate::scopes::ScopeState::scope_id`] method.
///
/// This method should be used when you want to schedule an update for a component
pub fn schedule_update_any() -> Option<Arc<dyn Fn(ScopeId) + Send + Sync>> {
    with_current_scope(|cx| cx.schedule_update_any())
}

/// Get the current scope id
pub fn current_scope_id() -> Option<ScopeId> {
    with_runtime(|rt| rt.current_scope_id()).flatten()
}

#[doc(hidden)]
/// Check if the virtual dom is currently inside of the body of a component
pub fn vdom_is_rendering() -> bool {
    with_runtime(|rt| rt.rendering.get()).unwrap_or_default()
}

/// Consume context from the current scope
pub fn consume_context<T: 'static + Clone>() -> Option<T> {
    with_current_scope(|cx| cx.consume_context::<T>()).flatten()
}

/// Consume context from the current scope
pub fn consume_context_from_scope<T: 'static + Clone>(scope_id: ScopeId) -> Option<T> {
    with_runtime(|rt| {
        rt.get_context(scope_id)
            .and_then(|cx| cx.consume_context::<T>())
    })
    .flatten()
}

/// Check if the current scope has a context
pub fn has_context<T: 'static + Clone>() -> Option<T> {
    with_current_scope(|cx| cx.has_context::<T>()).flatten()
}

/// Provide context to the current scope
pub fn provide_context<T: 'static + Clone>(value: T) -> Option<T> {
    with_current_scope(|cx| cx.provide_context(value))
}

/// Provide context to the the given scope
pub fn provide_context_to_scope<T: 'static + Clone>(scope_id: ScopeId, value: T) -> Option<T> {
    with_runtime(|rt| rt.get_context(scope_id).map(|cx| cx.provide_context(value))).flatten()
}

/// Provide a context to the root scope
pub fn provide_root_context<T: 'static + Clone>(value: T) -> Option<T> {
    with_current_scope(|cx| cx.provide_root_context(value))
}

/// Suspends the current component
pub fn suspend() -> Option<Element<'static>> {
    with_current_scope(|cx| {
        cx.suspend();
    });
    None
}

/// Throw an error into the nearest error boundary
pub fn throw(error: impl Debug + 'static) -> Option<()> {
    with_current_scope(|cx| cx.throw(error)).flatten()
}

/// Pushes the future onto the poll queue to be polled after the component renders.
pub fn push_future(fut: impl Future<Output = ()> + 'static) -> Option<TaskId> {
    with_current_scope(|cx| cx.push_future(fut))
}

/// Spawns the future but does not return the [`TaskId`]
pub fn spawn(fut: impl Future<Output = ()> + 'static) {
    with_current_scope(|cx| cx.spawn(fut));
}

/// Spawn a future that Dioxus won't clean up when this component is unmounted
///
/// This is good for tasks that need to be run after the component has been dropped.
pub fn spawn_forever(fut: impl Future<Output = ()> + 'static) -> Option<TaskId> {
    with_current_scope(|cx| cx.spawn_forever(fut))
}

/// Informs the scheduler that this task is no longer needed and should be removed.
///
/// This drops the task immediately.
pub fn remove_future(id: TaskId) {
    with_current_scope(|cx| cx.remove_future(id));
}
