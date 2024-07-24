use crate::innerlude::{DirtyTasks, Effect};
use crate::scope_context::SuspenseLocation;
use crate::{
    innerlude::{LocalTask, SchedulerMsg},
    scope_context::Scope,
    scopes::ScopeId,
    Task,
};
use slotmap::DefaultKey;
use std::collections::BTreeSet;
use std::fmt;
use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};

thread_local! {
    static RUNTIMES: RefCell<Vec<Rc<Runtime>>> = const { RefCell::new(vec![]) };
}

/// A global runtime that is shared across all scopes that provides the async runtime and context API
pub struct Runtime {
    pub(crate) scope_states: RefCell<Vec<Option<Scope>>>,

    // We use this to track the current scope
    // This stack should only be modified through [`Runtime::with_scope_on_stack`] to ensure that the stack is correctly restored
    scope_stack: RefCell<Vec<ScopeId>>,

    // We use this to track the current suspense location. Generally this lines up with the scope stack, but it may be different for children of a suspense boundary
    // This stack should only be modified through [`Runtime::with_suspense_location`] to ensure that the stack is correctly restored
    suspense_stack: RefCell<Vec<SuspenseLocation>>,

    // We use this to track the current task
    pub(crate) current_task: Cell<Option<Task>>,

    /// Tasks created with cx.spawn
    pub(crate) tasks: RefCell<slotmap::SlotMap<DefaultKey, Rc<LocalTask>>>,

    // Currently suspended tasks
    pub(crate) suspended_tasks: Cell<usize>,

    pub(crate) rendering: Cell<bool>,

    pub(crate) sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    // The effects that need to be run after the next render
    pub(crate) pending_effects: RefCell<BTreeSet<Effect>>,

    // Tasks that are waiting to be polled
    pub(crate) dirty_tasks: RefCell<BTreeSet<DirtyTasks>>,
}

impl Runtime {
    pub(crate) fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Rc<Self> {
        Rc::new(Self {
            sender,
            rendering: Cell::new(true),
            scope_states: Default::default(),
            scope_stack: Default::default(),
            suspense_stack: Default::default(),
            current_task: Default::default(),
            tasks: Default::default(),
            suspended_tasks: Default::default(),
            pending_effects: Default::default(),
            dirty_tasks: Default::default(),
        })
    }

    /// Get the current runtime
    pub fn current() -> Result<Rc<Self>, RuntimeError> {
        RUNTIMES
            .with(|stack| stack.borrow().last().cloned())
            .ok_or(RuntimeError::new())
    }

    /// Create a scope context. This slab is synchronized with the scope slab.
    pub(crate) fn create_scope(&self, context: Scope) {
        let id = context.id;
        let mut scopes = self.scope_states.borrow_mut();
        if scopes.len() <= id.0 {
            scopes.resize_with(id.0 + 1, Default::default);
        }
        scopes[id.0] = Some(context);
    }

    pub(crate) fn remove_scope(self: &Rc<Self>, id: ScopeId) {
        {
            let borrow = self.scope_states.borrow();
            if let Some(scope) = &borrow[id.0] {
                // Manually drop tasks, hooks, and contexts inside of the runtime
                self.on_scope(id, || {
                    // Drop all spawned tasks - order doesn't matter since tasks don't rely on eachother
                    // In theory nested tasks might not like this
                    for id in scope.spawned_tasks.take() {
                        self.remove_task(id);
                    }

                    // Drop all hooks in reverse order in case a hook depends on another hook.
                    for hook in scope.hooks.take().drain(..).rev() {
                        drop(hook);
                    }

                    // Drop all contexts
                    scope.shared_contexts.take();
                });
            }
        }
        self.scope_states.borrow_mut()[id.0].take();
    }

    /// Get the current scope id
    pub(crate) fn current_scope_id(&self) -> Result<ScopeId, RuntimeError> {
        self.scope_stack
            .borrow()
            .last()
            .copied()
            .ok_or(RuntimeError { _priv: () })
    }

    /// Call this function with the current scope set to the given scope
    ///
    /// Useful in a limited number of scenarios
    pub fn on_scope<O>(self: &Rc<Self>, id: ScopeId, f: impl FnOnce() -> O) -> O {
        let _runtime_guard = RuntimeGuard::new(self.clone());
        {
            self.push_scope(id);
        }
        let o = f();
        {
            self.pop_scope();
        }
        o
    }

    /// Get the current suspense location
    pub(crate) fn current_suspense_location(&self) -> Option<SuspenseLocation> {
        self.suspense_stack.borrow().last().cloned()
    }

    /// Run a callback a [`SuspenseLocation`] at the top of the stack
    pub(crate) fn with_suspense_location<O>(
        &self,
        suspense_location: SuspenseLocation,
        f: impl FnOnce() -> O,
    ) -> O {
        self.suspense_stack.borrow_mut().push(suspense_location);
        let o = f();
        self.suspense_stack.borrow_mut().pop();
        o
    }

    /// Run a callback with the current scope at the top of the stack
    pub(crate) fn with_scope_on_stack<O>(&self, scope: ScopeId, f: impl FnOnce() -> O) -> O {
        self.push_scope(scope);
        let o = f();
        self.pop_scope();
        o
    }

    /// Push a scope onto the stack
    fn push_scope(&self, scope: ScopeId) {
        let suspense_location = self
            .scope_states
            .borrow()
            .get(scope.0)
            .and_then(|s| s.as_ref())
            .map(|s| s.suspense_location())
            .unwrap_or_default();
        self.suspense_stack.borrow_mut().push(suspense_location);
        self.scope_stack.borrow_mut().push(scope);
    }

    /// Pop a scope off the stack
    fn pop_scope(&self) {
        self.scope_stack.borrow_mut().pop();
        self.suspense_stack.borrow_mut().pop();
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn get_state(&self, id: ScopeId) -> Option<Ref<'_, Scope>> {
        Ref::filter_map(self.scope_states.borrow(), |contexts| {
            contexts.get(id.0).and_then(|f| f.as_ref())
        })
        .ok()
    }

    /// Pushes a new scope onto the stack
    pub(crate) fn push(runtime: Rc<Runtime>) {
        RUNTIMES.with(|stack| stack.borrow_mut().push(runtime));
    }

    /// Pops a scope off the stack
    pub(crate) fn pop() {
        RUNTIMES.with(|stack| stack.borrow_mut().pop());
    }

    /// Runs a function with the current runtime
    pub(crate) fn with<R>(f: impl FnOnce(&Runtime) -> R) -> Result<R, RuntimeError> {
        Self::current().map(|r| f(&r))
    }

    /// Runs a function with the current scope
    pub(crate) fn with_current_scope<R>(f: impl FnOnce(&Scope) -> R) -> Result<R, RuntimeError> {
        Self::with(|rt| {
            rt.current_scope_id()
                .ok()
                .and_then(|scope| rt.get_state(scope).map(|sc| f(&sc)))
        })
        .ok()
        .flatten()
        .ok_or(RuntimeError::new())
    }

    /// Runs a function with the current scope
    pub(crate) fn with_scope<R>(
        scope: ScopeId,
        f: impl FnOnce(&Scope) -> R,
    ) -> Result<R, RuntimeError> {
        Self::with(|rt| rt.get_state(scope).map(|sc| f(&sc)))
            .ok()
            .flatten()
            .ok_or(RuntimeError::new())
    }

    /// Finish a render. This will mark all effects as ready to run and send the render signal.
    pub(crate) fn finish_render(&self) {
        // If there are new effects we can run, send a message to the scheduler to run them (after the renderer has applied the mutations)
        if !self.pending_effects.borrow().is_empty() {
            self.sender
                .unbounded_send(SchedulerMsg::EffectQueued)
                .expect("Scheduler should exist");
        }
    }

    /// Check if we should render a scope
    pub(crate) fn scope_should_render(&self, scope_id: ScopeId) -> bool {
        // If there are no suspended futures, we know the scope is not  and we can skip context checks
        if self.suspended_tasks.get() == 0 {
            return true;
        }
        // If this is not a suspended scope, and we are under a frozen context, then we should
        let scopes = self.scope_states.borrow();
        let scope = &scopes[scope_id.0].as_ref().unwrap();
        !matches!(scope.suspense_location(), SuspenseLocation::UnderSuspense(suspense) if suspense.has_suspended_tasks())
    }
}

/// A guard for a new runtime. This must be used to override the current runtime when importing components from a dynamic library that has it's own runtime.
///
/// ```rust
/// use dioxus::prelude::*;
///
/// fn main() {
///     let virtual_dom = VirtualDom::new(app);
/// }
///
/// fn app() -> Element {
///     rsx!{ Component { runtime: Runtime::current().unwrap() } }
/// }
///
/// // In a dynamic library
/// #[derive(Props, Clone)]
/// struct ComponentProps {
///    runtime: std::rc::Rc<Runtime>,
/// }
///
/// impl PartialEq for ComponentProps {
///     fn eq(&self, _other: &Self) -> bool {
///         true
///     }
/// }
///
/// fn Component(cx: ComponentProps) -> Element {
///     use_hook(|| {
///         let _guard = RuntimeGuard::new(cx.runtime.clone());
///     });
///
///     rsx! { div {} }
/// }
/// ```
pub struct RuntimeGuard(());

impl RuntimeGuard {
    /// Create a new runtime guard that sets the current Dioxus runtime. The runtime will be reset when the guard is dropped
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Runtime::push(runtime);
        Self(())
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        Runtime::pop();
    }
}

/// Missing Dioxus runtime error.
pub struct RuntimeError {
    _priv: (),
}

impl RuntimeError {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self { _priv: () }
    }
}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeError").finish()
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Must be called from inside a Dioxus runtime.

Help: Some APIs in dioxus require a global runtime to be present.
If you are calling one of these APIs from outside of a dioxus runtime
(typically in a web-sys closure or dynamic library), you will need to
grab the runtime from a scope that has it and then move it into your
new scope with a runtime guard.

For example, if you are trying to use dioxus apis from a web-sys
closure, you can grab the runtime from the scope it is created in:

```rust
use dioxus::prelude::*;
static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

#[component]
fn MyComponent() -> Element {{
    use_effect(|| {{
        // Grab the runtime from the MyComponent scope
        let runtime = Runtime::current().expect(\"Components run in the Dioxus runtime\");
        // Move the runtime into the web-sys closure scope
        let web_sys_closure = Closure::new(|| {{
            // Then create a guard to provide the runtime to the closure
            let _guard = RuntimeGuard::new(runtime);
            // and run whatever code needs the runtime
            tracing::info!(\"The count is: {{COUNT}}\");
        }});
    }})
}}
```"
        )
    }
}

impl std::error::Error for RuntimeError {}
