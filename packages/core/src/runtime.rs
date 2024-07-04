use slotmap::DefaultKey;

use crate::innerlude::Effect;
use crate::{
    innerlude::{LocalTask, SchedulerMsg},
    render_signal::RenderSignal,
    scope_context::Scope,
    scopes::ScopeId,
    Task,
};
use std::collections::BTreeSet;
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
    pub(crate) scope_stack: RefCell<Vec<ScopeId>>,

    // We use this to track the current task
    pub(crate) current_task: Cell<Option<Task>>,

    /// Tasks created with cx.spawn
    pub(crate) tasks: RefCell<slotmap::SlotMap<DefaultKey, Rc<LocalTask>>>,

    // Currently suspended tasks
    pub(crate) suspended_tasks: Cell<usize>,

    pub(crate) rendering: Cell<bool>,

    pub(crate) sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>,

    // Synchronous tasks need to be run after the next render. The virtual dom stores a list of those tasks to send a signal to them when the next render is done.
    pub(crate) render_signal: RenderSignal,

    // The effects that need to be run after the next render
    pub(crate) pending_effects: RefCell<BTreeSet<Effect>>,
}

impl Runtime {
    pub(crate) fn new(sender: futures_channel::mpsc::UnboundedSender<SchedulerMsg>) -> Rc<Self> {
        Rc::new(Self {
            sender,
            render_signal: RenderSignal::default(),
            rendering: Cell::new(true),
            scope_states: Default::default(),
            scope_stack: Default::default(),
            current_task: Default::default(),
            tasks: Default::default(),
            suspended_tasks: Default::default(),
            pending_effects: Default::default(),
        })
    }

    /// Get the current runtime
    pub fn current() -> Option<Rc<Self>> {
        RUNTIMES.with(|stack| stack.borrow().last().cloned())
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
                let _runtime_guard = RuntimeGuard::new(self.clone());
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
    pub(crate) fn current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().copied()
    }

    /// Call this function with the current scope set to the given scope
    ///
    /// Useful in a limited number of scenarios
    pub fn on_scope<O>(&self, id: ScopeId, f: impl FnOnce() -> O) -> O {
        {
            self.scope_stack.borrow_mut().push(id);
        }
        let o = f();
        {
            self.scope_stack.borrow_mut().pop();
        }
        o
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
    pub(crate) fn with<R>(f: impl FnOnce(&Runtime) -> R) -> Option<R> {
        RUNTIMES.with(|stack| stack.borrow().last().map(|r| f(r)))
    }

    /// Runs a function with the current scope
    pub(crate) fn with_current_scope<R>(f: impl FnOnce(&Scope) -> R) -> Option<R> {
        Self::with(|rt| {
            rt.current_scope_id()
                .and_then(|scope| rt.get_state(scope).map(|sc| f(&sc)))
        })
        .flatten()
    }

    /// Runs a function with the current scope
    pub(crate) fn with_scope<R>(scope: ScopeId, f: impl FnOnce(&Scope) -> R) -> Option<R> {
        Self::with(|rt| rt.get_state(scope).map(|sc| f(&sc))).flatten()
    }

    /// Finish a render. This will mark all effects as ready to run and send the render signal.
    pub(crate) fn finish_render(&self) {
        // If there are new effects we can run, send a message to the scheduler to run them (after the renderer has applied the mutations)
        if !self.pending_effects.borrow().is_empty() {
            self.sender
                .unbounded_send(SchedulerMsg::EffectQueued)
                .expect("Scheduler should exist");
        }

        // And send the render signal
        self.render_signal.send();
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
