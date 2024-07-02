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
//! fn ChildComponent(signal: Signal<Option<i32>>) -> Element {
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
//!         eval(format!(r#"document.getElementById("{id}").innerHTML = "Hello World";"#));
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

use crate::innerlude::Effect;
use crate::ScopeId;
use crate::Task;
use crate::VirtualDom;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;
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
    /// Queue a task to be polled
    pub(crate) fn queue_task(&mut self, task: Task, order: ScopeOrder) {
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
        self.dirty_scopes.insert(order);
    }

    /// Check if there are any dirty scopes
    pub(crate) fn has_dirty_scopes(&self) -> bool {
        !self.dirty_scopes.is_empty()
    }

    /// Take the top task from the highest scope
    pub(crate) fn pop_task(&mut self) -> Option<Task> {
        let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
        let mut tasks = dirty_tasks.first()?;

        // If the scope doesn't exist for whatever reason, then we should skip it
        while !self.scopes.contains(tasks.order.id.0) {
            dirty_tasks.pop_first();
            tasks = dirty_tasks.first()?;
        }

        let mut tasks = tasks.tasks_queued.borrow_mut();
        let task = tasks.pop_front()?;
        if tasks.is_empty() {
            drop(tasks);
            dirty_tasks.pop_first();
        }
        Some(task)
    }

    /// Take any effects from the highest scope. This should only be called if there is no pending scope reruns or tasks
    pub(crate) fn pop_effect(&mut self) -> Option<Effect> {
        let mut pending_effects = self.runtime.pending_effects.borrow_mut();
        let mut effect = pending_effects.pop_first()?;

        // If the scope doesn't exist for whatever reason, then we should skip it
        while !self.scopes.contains(effect.order.id.0) {
            effect = pending_effects.pop_first()?;
        }

        Some(effect)
    }

    /// Take any work from the highest scope. This may include rerunning the scope and/or running tasks
    pub(crate) fn pop_work(&mut self) -> Option<Work> {
        let mut dirty_scope = self.dirty_scopes.first();
        // Pop any invalid scopes off of each dirty task;
        while let Some(scope) = dirty_scope {
            if !self.scopes.contains(scope.id.0) {
                self.dirty_scopes.pop_first();
                dirty_scope = self.dirty_scopes.first();
            } else {
                break;
            }
        }

        // Find the height of the highest dirty scope
        let dirty_task = {
            let mut dirty_tasks = self.runtime.dirty_tasks.borrow_mut();
            let mut dirty_task = dirty_tasks.first();
            // Pop any invalid tasks off of each dirty scope;
            while let Some(task) = dirty_task {
                if task.tasks_queued.borrow().is_empty() || !self.scopes.contains(task.order.id.0) {
                    dirty_tasks.pop_first();
                    dirty_task = dirty_tasks.first()
                } else {
                    break;
                }
            }
            dirty_task.map(|task| task.order)
        };

        match (dirty_scope, dirty_task) {
            (Some(scope), Some(task)) => {
                let tasks_order = task.borrow();
                match scope.cmp(tasks_order) {
                    std::cmp::Ordering::Less => {
                        let scope = self.dirty_scopes.pop_first().unwrap();
                        Some(Work::RerunScope(scope))
                    }
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                        Some(Work::PollTask(self.pop_task().unwrap()))
                    }
                }
            }
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
pub enum Work {
    RerunScope(ScopeOrder),
    PollTask(Task),
}

#[derive(Debug, Clone, Eq)]
pub(crate) struct DirtyTasks {
    pub order: ScopeOrder,
    pub tasks_queued: RefCell<VecDeque<Task>>,
}

impl From<ScopeOrder> for DirtyTasks {
    fn from(order: ScopeOrder) -> Self {
        Self {
            order,
            tasks_queued: VecDeque::new().into(),
        }
    }
}

impl DirtyTasks {
    pub fn queue_task(&self, task: Task) {
        let mut borrow_mut = self.tasks_queued.borrow_mut();
        // If the task is already queued, we don't need to do anything
        if borrow_mut.contains(&task) {
            return;
        }
        borrow_mut.push_back(task);
    }

    pub(crate) fn remove(&self, id: Task) {
        self.tasks_queued.borrow_mut().retain(|task| *task != id);
    }
}

impl Borrow<ScopeOrder> for DirtyTasks {
    fn borrow(&self) -> &ScopeOrder {
        &self.order
    }
}

impl PartialOrd for DirtyTasks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.order.cmp(&other.order))
    }
}

impl Ord for DirtyTasks {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialEq for DirtyTasks {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

impl Hash for DirtyTasks {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.order.hash(state);
    }
}
