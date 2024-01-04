use std::cell::{Cell, Ref, RefCell};

use crate::{innerlude::Scheduler, scope_context::ScopeContext, scopes::ScopeId};
use std::rc::Rc;

thread_local! {
    static RUNTIMES: RefCell<Vec<Rc<Runtime>>> = RefCell::new(vec![]);
}

/// Pushes a new scope onto the stack
pub(crate) fn push_runtime(runtime: Rc<Runtime>) {
    RUNTIMES.with(|stack| stack.borrow_mut().push(runtime));
}

/// Pops a scope off the stack
pub(crate) fn pop_runtime() {
    RUNTIMES.with(|stack| stack.borrow_mut().pop());
}

/// Runs a function with the current runtime
pub(crate) fn with_runtime<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Runtime) -> R,
{
    RUNTIMES.with(|stack| {
        let stack = stack.borrow();
        stack.last().map(|r| f(r))
    })
}

/// Runs a function with the current scope
pub(crate) fn with_current_scope<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ScopeContext) -> R,
{
    with_runtime(|runtime| {
        runtime
            .current_scope_id()
            .and_then(|scope| runtime.get_context(scope).map(|sc| f(&sc)))
    })
    .flatten()
}

/// A global runtime that is shared across all scopes that provides the async runtime and context API
pub struct Runtime {
    pub(crate) scope_contexts: RefCell<Vec<Option<ScopeContext>>>,
    pub(crate) scheduler: Rc<Scheduler>,

    // We use this to track the current scope
    pub(crate) scope_stack: RefCell<Vec<ScopeId>>,
    pub(crate) rendering: Cell<bool>,
}

impl Runtime {
    pub(crate) fn new(scheduler: Rc<Scheduler>) -> Rc<Self> {
        Rc::new(Self {
            scheduler,

            scope_contexts: Default::default(),

            scope_stack: Default::default(),

            rendering: Cell::new(true),
        })
    }

    /// Get the current runtime
    pub fn current() -> Option<Rc<Self>> {
        RUNTIMES.with(|stack| stack.borrow().last().cloned())
    }

    /// Create a scope context. This slab is synchronized with the scope slab.
    pub(crate) fn create_context_at(&self, id: ScopeId, context: ScopeContext) {
        let mut contexts = self.scope_contexts.borrow_mut();
        if contexts.len() <= id.0 {
            contexts.resize_with(id.0 + 1, Default::default);
        }
        contexts[id.0] = Some(context);
    }

    pub(crate) fn remove_context(&self, id: ScopeId) {
        self.scope_contexts.borrow_mut()[id.0] = None;
    }

    /// Get the current scope id
    pub(crate) fn current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().copied()
    }

    /// Get the context for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub(crate) fn get_context(&self, id: ScopeId) -> Option<Ref<'_, ScopeContext>> {
        Ref::filter_map(self.scope_contexts.borrow(), |contexts| {
            contexts.get(id.0).and_then(|f| f.as_ref())
        })
        .ok()
    }
}

/// A gaurd for a new runtime. This must be used to override the current runtime when importing components from a dynamic library that has it's own runtime.
///
/// ```rust
/// use dioxus::prelude::*;
///
/// fn main() {
///     let virtual_dom = VirtualDom::new(app);
/// }
///
/// fn app(cx: Scope) -> Element {
///     render!{ Component { runtime: Runtime::current().unwrap() } }
/// }
///
/// // In a dynamic library
/// #[derive(Props)]
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
/// fn Component(cx: Scope<ComponentProps>) -> Element {
///     cx.use_hook(|| RuntimeGuard::new(cx.props.runtime.clone()));
///
///     render! { div {} }
/// }
/// ```
pub struct RuntimeGuard(Rc<Runtime>);

impl RuntimeGuard {
    /// Create a new runtime guard that sets the current Dioxus runtime. The runtime will be reset when the guard is dropped
    pub fn new(runtime: Rc<Runtime>) -> Self {
        push_runtime(runtime.clone());
        Self(runtime)
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        pop_runtime();
    }
}
