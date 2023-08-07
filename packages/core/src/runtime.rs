use std::cell::{Ref, RefCell};

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
        stack.last().map(|r| f(&**r))
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
            .and_then(|scope| runtime.get_context(scope).map(|sc| f(&*sc)))
    })
    .flatten()
}

pub struct Runtime {
    pub(crate) scope_contexts: RefCell<Vec<Option<ScopeContext>>>,
    pub(crate) scheduler: Rc<Scheduler>,

    // We use this to track the current scope
    pub(crate) scope_stack: RefCell<Vec<ScopeId>>,
}

impl Runtime {
    pub(crate) fn new(scheduler: Rc<Scheduler>) -> Rc<Self> {
        let runtime = Rc::new(Self {
            scheduler,

            scope_contexts: Default::default(),

            scope_stack: Default::default(),
        });
        runtime
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
    pub fn current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().copied()
    }

    /// Get the context for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub fn get_context(&self, id: ScopeId) -> Option<Ref<'_, ScopeContext>> {
        Ref::filter_map(self.scope_contexts.borrow(), |contexts| {
            contexts.get(id.0).and_then(|f| f.as_ref())
        })
        .ok()
    }
}

pub struct RuntimeGuard(Rc<Runtime>);

impl RuntimeGuard {
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
