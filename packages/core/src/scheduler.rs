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
mod message;
mod queues;
pub(crate) use message::SchedulerMsg;
pub use queues::*;

impl VirtualDom {
    /// Queue a task to be polled
    pub(crate) fn queue_task(&mut self, task: Task, order: ScopeOrder) {
        if !self.scope_has_live_parent_chain(order.id) {
            return;
        }
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        match dirty_tasks.get(&order) {
            Some(scope) => scope.queue_task(task),
            None => {
                let scope = DirtyTasks::from(order);
                scope.queue_task(task);
                dirty_tasks.insert(scope);
            }
        }
    }

    /// Queue a scope to be rerendered
    pub(crate) fn queue_scope(&mut self, order: ScopeOrder) {
        if !self.scope_has_live_parent_chain(order.id) {
            return;
        }
        self.dirty_scopes.insert(order);
    }

    /// Pop the smallest-height dirty scope that is a strict descendant of
    /// `ancestor`. Suspense boundaries use this to flush queued child renders
    /// inline, so newly suspended futures are visible before the boundary
    /// commits its diff.
    pub(crate) fn pop_dirty_descendant_of(&mut self, ancestor: ScopeId) -> Option<ScopeOrder> {
        let next = self
            .dirty_scopes
            .iter()
            .copied()
            .filter(|order| {
                order.id != ancestor
                    && self.scope_has_live_parent_chain(order.id)
                    && self.runtime.is_descendant_of(order.id, ancestor)
            })
            .min_by(|left, right| left.height.cmp(&right.height).then(left.id.cmp(&right.id)))?;
        self.dirty_scopes.remove(&next);
        Some(next)
    }

    /// Check if there are any dirty scopes
    pub fn has_dirty_scopes(&self) -> bool {
        !self.dirty_scopes.is_empty()
    }

    /// Walk up the parent chain and confirm every ancestor scope is still
    /// alive. A scope whose parent was dropped should never be rerun.
    fn scope_has_live_parent_chain(&self, scope_id: ScopeId) -> bool {
        let mut current = scope_id;
        while let Some(state) = self.runtime.try_get_state(current) {
            let parent = state.parent_id();
            drop(state);
            let Some(parent) = parent else { break };
            if self.runtime.try_get_state(parent).is_none() {
                return false;
            }
            current = parent;
        }
        self.runtime.try_get_state(scope_id).is_some()
    }

    fn first_dirty_scope(&mut self) -> Option<ScopeOrder> {
        loop {
            let order = self.dirty_scopes.first()?;
            if self.scope_has_live_parent_chain(order.id) {
                return Some(order);
            }
            self.dirty_scopes.pop_first();
        }
    }

    /// Take the top task from the highest scope
    pub(crate) fn pop_task(&mut self) -> Option<Task> {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let tasks = dirty_tasks.first()?;

        // The scope that owns the effect should still exist. We can't just ignore the task if the scope doesn't exist
        // because the scope id may have been reallocated
        debug_assert!(self.scopes.contains(tasks.order.id.0));

        let mut tasks = tasks.tasks_queued.borrow_mut();
        let task = tasks.pop_front()?;
        if tasks.is_empty() {
            drop(tasks);
            dirty_tasks.pop_first();
        }
        Some(task)
    }

    fn first_dirty_task_order(&mut self) -> Option<ScopeOrder> {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let mut dirty_task = dirty_tasks.first();
        // Pop any invalid tasks off of each dirty scope
        while let Some(task) = dirty_task {
            if task.tasks_queued.borrow().is_empty()
                || !self.scope_has_live_parent_chain(task.order.id)
            {
                dirty_tasks.pop_first();
                dirty_task = dirty_tasks.first();
            } else {
                break;
            }
        }
        dirty_task.map(|task| task.order)
    }

    /// Take any work from the highest scope. This may include rerunning the scope and/or running tasks
    pub(crate) fn pop_work(&mut self) -> Option<Work> {
        let dirty_scope = self.first_dirty_scope();
        // Make sure the top dirty scope is valid
        #[cfg(debug_assertions)]
        if let Some(scope) = &dirty_scope {
            assert!(self.scopes.contains(scope.id.0));
            assert!(self.scope_has_live_parent_chain(scope.id));
        }

        let dirty_task = self.first_dirty_task_order();

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
