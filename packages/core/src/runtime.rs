use std::cell::RefCell;

use crate::{
    innerlude::{DirtyScope, Scheduler, SchedulerMsg},
    scope_context::ScopeContext,
    scopes::{ScopeId, ScopeState},
};
use rustc_hash::FxHashSet;
use slab::Slab;
use std::{collections::BTreeSet, rc::Rc};

thread_local! {
    static RUNTIMES: RefCell<Vec<Runtime>> = RefCell::new(vec![]);
}

/// Pushes a new scope onto the stack
pub(crate) fn push_runtime(runtime: Runtime) {
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
        stack.last().map(f)
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
            .and_then(|scope| runtime.get_context(scope).map(f))
    })
    .flatten()
}

pub struct Runtime {
    pub(crate) scope_contexts: Slab<ScopeContext>,
    pub(crate) scheduler: Rc<Scheduler>,

    // While diffing we need some sort of way of breaking off a stream of suspended mutations.
    pub(crate) scope_stack: RefCell<Vec<ScopeId>>,
}

impl Runtime {
    pub(crate) fn new(scheduler: Rc<Scheduler>) -> Rc<Self> {
        Rc::new(Self {
            scheduler,

            scope_contexts: Default::default(),

            scope_stack: Default::default(),
        })
    }

    /// Get the current scope id
    pub fn current_scope_id(&self) -> Option<ScopeId> {
        self.scope_stack.borrow().last().copied()
    }

    /// Get the state for any scope given its ID
    ///
    /// This is useful for inserting or removing contexts from a scope, or rendering out its root node
    pub fn get_context(&self, id: ScopeId) -> Option<&ScopeContext> {
        self.scope_contexts.get(id.0).map(|f| &*f)
    }

    /// Get the single scope at the top of the Runtime tree that will always be around
    ///
    /// This scope has a ScopeId of 0 and is the root of the tree
    pub fn base_context(&self) -> &ScopeContext {
        self.get_context(ScopeId(0)).unwrap()
    }
}
