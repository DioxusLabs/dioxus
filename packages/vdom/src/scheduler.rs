//! # Dioxus uses a scheduler to run queued work in the correct order.
//!
//! ## Goals
//! We try to prevent three different situations:
//! 1. Running queued work after it could be dropped. Related issues (<https://github.com/DioxusLabs/dioxus/pull/1993>)
//!
//! User code often assumes that this property is true. For example, if this code reruns the child component after signal is changed to None, it will panic
//! ```rust, ignore
//! fn ParentComponent() -> Element {
//!     let signal: Signal<Option<i32>> = use_signal(None);
//!
//!     rsx! {
//!         if signal.read().is_some() {
//!             ChildComponent { signal }
//!         }
//!     }
//! }
//!
//! #[component]
//! fn ChildComponent(signal: WriteSignal<Option<i32>>) -> Element {
//!     // It feels safe to assume that signal is some because the parent component checked that it was some
//!     rsx! { "{signal.read().unwrap()}" }
//! }
//! ```
//!
//! 2. Running effects before the dom is updated. Related issues (<https://github.com/DioxusLabs/dioxus/issues/2307>)
//!
//! Effects can be used to run code that accesses the DOM directly. They should only run when the DOM is in an updated state. If they are run with an out of date version of the DOM, unexpected behavior can occur.
//! ```rust, ignore
//! fn EffectComponent() -> Element {
//!     let id = use_signal(0);
//!     use_effect(move || {
//!         let id = id.read();
//!         // This will panic if the id is not written to the DOM before the effect is run
//!         document::eval(format!(r#"document.getElementById("{id}").innerHTML = "Hello World";"#));
//!     });
//!
//!     rsx! {
//!         div { id: "{id}" }
//!     }
//! }
//! ```
//!
//! 3. Observing out of date state. Related issues (<https://github.com/DioxusLabs/dioxus/issues/1935>)
//!
//! Where ever possible, updates should happen in an order that makes it impossible to observe an out of date state.
//! ```rust, ignore
//! fn OutOfDateComponent() -> Element {
//!     let id = use_signal(0);
//!     // When you read memo, it should **always** be two times the value of id
//!     let memo = use_memo(move || id() * 2);
//!     assert_eq!(memo(), id() * 2);
//!
//!     // This should be true even if you update the value of id in the middle of the component
//!     id += 1;
//!     assert_eq!(memo(), id() * 2);
//!
//!     rsx! {
//!         div { id: "{id}" }
//!     }
//! }
//! ```
//!
//! ## Implementation
//!
//! There are three different types of queued work that can be run by the virtualdom:
//! 1. Dirty Scopes:
//!    Description: When a scope is marked dirty, a rerun of the scope will be scheduled. This will cause the scope to rerun and update the DOM if any changes are detected during the diffing phase.
//!    Priority: These are the highest priority tasks. Dirty scopes will be rerun in order from the scope closest to the root to the scope furthest from the root. We follow this order to ensure that if a higher component reruns and drops a lower component, the lower component will not be run after it should be dropped.
//!
//! 2. Tasks:
//!    Description: Futures spawned in the dioxus runtime each have an unique task id. When the waker for that future is called, the task is rerun.
//!    Priority: These are the second highest priority tasks. They are run after all other dirty scopes have been resolved because those dirty scopes may cause children (and the tasks those children own) to drop which should cancel the futures.
//!
//! 3. Effects:
//!    Description: Effects should always run after all changes to the DOM have been applied.
//!    Priority: These are the lowest priority tasks in the scheduler. They are run after all other dirty scopes and futures have been resolved. Other tasks may cause components to rerun, which would update the DOM. These effects should only run after the DOM has been updated.

use crate::ScopeId;
use crate::Task;
use crate::VirtualDom;
use crate::innerlude::Effect;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Eq)]
pub struct ScopeOrder {
    pub(crate) height: u32,
    pub(crate) id: ScopeId,
}

impl ScopeOrder {
    pub fn new(height: u32, id: ScopeId) -> Self {
        Self { height, id }
    }
}

impl PartialEq for ScopeOrder {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for ScopeOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScopeOrder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height).then(self.id.cmp(&other.id))
    }
}

impl Hash for ScopeOrder {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl VirtualDom {
    fn remove_empty_dirty_tasks(&mut self) {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        while dirty_tasks
            .first_key_value()
            .is_some_and(|(_, tasks)| tasks.is_empty())
        {
            dirty_tasks.pop_first();
        }
    }

    fn scope_order(&self, id: ScopeId) -> Option<ScopeOrder> {
        self.runtime
            .try_get_state(id)
            .map(|scope| ScopeOrder::new(scope.height(), id))
    }

    /// Queue a task to be polled
    pub(crate) fn queue_task(&mut self, task: Task, order: ScopeOrder) {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let tasks = dirty_tasks.entry(order).or_default();
        if !tasks.contains(&task) {
            tasks.push_back(task);
        }
    }

    /// Queue a scope to be rerendered
    pub(crate) fn queue_scope(&mut self, id: ScopeId) -> bool {
        let Some(order) = self.scope_order(id) else {
            return false;
        };

        self.dirty_scopes.insert(order)
    }

    /// Remove a scope from the rerender queue
    pub(crate) fn mark_clean(&mut self, id: ScopeId) -> bool {
        self.scope_order(id)
            .is_some_and(|order| self.dirty_scopes.remove(&order))
    }

    /// Pop the highest-priority dirty scope below `ancestor`.
    pub(crate) fn pop_dirty_descendant_scope(&mut self, ancestor: ScopeId) -> Option<ScopeOrder> {
        let order = self
            .dirty_scopes
            .iter()
            .find(|order| self.runtime.is_descendant_of(order.id, ancestor))
            .copied()?;
        self.dirty_scopes.take(&order)
    }

    /// Check if there are any dirty scopes
    pub(crate) fn has_dirty_scopes(&self) -> bool {
        !self.dirty_scopes.is_empty()
    }

    /// Take the top task from the highest scope
    pub(crate) fn pop_task(&mut self) -> Option<Task> {
        self.remove_empty_dirty_tasks();

        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let mut entry = dirty_tasks.first_entry()?;
        let order = *entry.key();

        // The scope that owns the effect should still exist. We can't just ignore the task if the scope doesn't exist
        // because the scope id may have been reallocated
        debug_assert!(self.scopes.contains(order.id.index()));

        let tasks = entry.get_mut();
        let task = tasks.pop_front()?;
        if tasks.is_empty() {
            entry.remove_entry();
        }
        Some(task)
    }

    /// Take any effects from the highest scope. This should only be called if there are no pending scope reruns or tasks.
    pub(crate) fn pop_effect(&mut self) -> Option<Effect> {
        let mut pending_effects = self.runtime.pending_effects.borrow_mut();
        let effect = pending_effects.pop_first()?;

        // The scope that owns the effect should still exist. We can't just ignore the effect if the scope doesn't exist
        // because the scope id may have been reallocated.
        debug_assert!(self.scopes.contains(effect.order.id.index()));

        Some(effect)
    }

    /// Take any work from the highest scope. This may include rerunning the scope and/or running tasks
    pub(crate) fn pop_work(&mut self) -> Option<Work> {
        let dirty_scope = self.dirty_scopes.first().copied();
        // Make sure the top dirty scope is valid
        #[cfg(debug_assertions)]
        if let Some(scope) = dirty_scope.as_ref() {
            assert!(self.scopes.contains(scope.id.index()));
        }

        self.remove_empty_dirty_tasks();
        let dirty_task = self
            .runtime
            .dirty_tasks
            .borrow()
            .first_key_value()
            .map(|(order, _)| *order);

        match (dirty_scope, dirty_task) {
            (Some(scope), Some(task)) => match scope.cmp(&task) {
                std::cmp::Ordering::Less => {
                    let scope = self.dirty_scopes.pop_first().unwrap();
                    Some(Work::RerunScope(scope))
                }
                std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                    Some(Work::PollTask(self.pop_task().unwrap()))
                }
            },
            (Some(_), None) => {
                let scope = self.dirty_scopes.pop_first().unwrap();
                Some(Work::RerunScope(scope))
            }
            (None, Some(_)) => Some(Work::PollTask(self.pop_task().unwrap())),
            (None, None) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum Work {
    RerunScope(ScopeOrder),
    PollTask(Task),
}
