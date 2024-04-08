//! Dioxus resolves scopes in a specific order to avoid unexpected behavior. All tasks are resolved in the order of height. Scopes that are higher up in the tree are resolved first.
//! When a scope that is higher up in the tree is rerendered, it may drop scopes lower in the tree along with their tasks.
//!
//! ```rust
//! use dioxus::prelude::*;
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
//! fn Child(vec: Signal<Vec<usize>>, idx: usize) -> Element {
//!     use_hook(move || {
//!         spawn(async move {
//!             // If we let this task run after the child is dropped, it will panic.
//!             println!("Task {}", vec.read()[idx]);
//!         });
//!     });
//!
//!     rsx! {}
//! }
//! ```

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
                            tasks: Default::default(),
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
                    tasks: Default::default(),
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
    pub tasks: VecDeque<Task>,
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
        self.tasks_queued.borrow_mut().push_back(task);
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
