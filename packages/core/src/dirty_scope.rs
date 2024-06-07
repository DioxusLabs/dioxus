//! Dioxus resolves scopes in a specific order to avoid unexpected behavior. All tasks are resolved in the order of height. Scopes that are higher up in the tree are resolved first.
//! When a scope that is higher up in the tree is rerendered, it may drop scopes lower in the tree along with their tasks.
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
//! fn app() -> Element {
//!     let vec = use_signal(|| vec![0; 10]);
//!     rsx! {
//!         // If the length of the vec shrinks we need to make sure that the children are dropped along with their tasks the new state of the vec is read
//!         for idx in 0..vec.len() {
//!             Child { idx, vec }
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

use crate::ScopeId;
use crate::Task;
use crate::VirtualDom;
use std::borrow::Borrow;
use std::cell::RefCell;
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
        match self.dirty_tasks.get(&order) {
            Some(scope) => scope.queue_task(task),
            None => {
                let scope = DirtyTasks::from(order);
                scope.queue_task(task);
                self.dirty_tasks.insert(scope);
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

    /// Take any tasks from the highest scope
    pub(crate) fn pop_task(&mut self) -> Option<DirtyTasks> {
        let mut task = self.dirty_tasks.pop_first()?;

        // If the scope doesn't exist for whatever reason, then we should skip it
        while !self.scopes.contains(task.order.id.0) {
            task = self.dirty_tasks.pop_first()?;
        }

        Some(task)
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

        let mut dirty_task = self.dirty_tasks.first();
        // Pop any invalid tasks off of each dirty scope;
        while let Some(task) = dirty_task {
            if !self.scopes.contains(task.order.id.0) {
                self.dirty_tasks.pop_first();
                dirty_task = self.dirty_tasks.first();
            } else {
                break;
            }
        }

        match (dirty_scope, dirty_task) {
            (Some(scope), Some(task)) => {
                let tasks_order = task.borrow();
                match scope.cmp(tasks_order) {
                    std::cmp::Ordering::Less => {
                        let scope = self.dirty_scopes.pop_first().unwrap();
                        Some(Work {
                            scope,
                            rerun_scope: true,
                            tasks: Vec::new(),
                        })
                    }
                    std::cmp::Ordering::Greater => {
                        let task = self.dirty_tasks.pop_first().unwrap();
                        Some(Work {
                            scope: task.order,
                            rerun_scope: false,
                            tasks: task.tasks_queued.into_inner(),
                        })
                    }
                    std::cmp::Ordering::Equal => {
                        let scope = self.dirty_scopes.pop_first().unwrap();
                        let task = self.dirty_tasks.pop_first().unwrap();
                        Some(Work {
                            scope,
                            rerun_scope: true,
                            tasks: task.tasks_queued.into_inner(),
                        })
                    }
                }
            }
            (Some(_), None) => {
                let scope = self.dirty_scopes.pop_first().unwrap();
                Some(Work {
                    scope,
                    rerun_scope: true,
                    tasks: Vec::new(),
                })
            }
            (None, Some(_)) => {
                let task = self.dirty_tasks.pop_first().unwrap();
                Some(Work {
                    scope: task.order,
                    rerun_scope: false,
                    tasks: task.tasks_queued.into_inner(),
                })
            }
            (None, None) => None,
        }
    }
}

#[derive(Debug)]
pub struct Work {
    pub scope: ScopeOrder,
    pub rerun_scope: bool,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Eq)]
pub(crate) struct DirtyTasks {
    pub order: ScopeOrder,
    pub tasks_queued: RefCell<Vec<Task>>,
}

impl From<ScopeOrder> for DirtyTasks {
    fn from(order: ScopeOrder) -> Self {
        Self {
            order,
            tasks_queued: Vec::new().into(),
        }
    }
}

impl DirtyTasks {
    pub fn queue_task(&self, task: Task) {
        self.tasks_queued.borrow_mut().push(task);
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
