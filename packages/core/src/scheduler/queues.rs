use crate::{ScopeId, Task};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
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

/// The set of scopes whose mounts are waiting to be diffed, ordered from the
/// scope closest to the root to the scope furthest from it.
#[derive(Debug, Default)]
pub(crate) struct DirtyScopes {
    scopes: BTreeSet<ScopeOrder>,
}

impl DirtyScopes {
    pub(crate) fn insert(&mut self, order: ScopeOrder) {
        self.scopes.insert(order);
    }

    pub(crate) fn remove(&mut self, order: &ScopeOrder) -> bool {
        self.scopes.remove(order)
    }

    /// Remove the exact ordered entry. Kept as a distinct name from
    /// [`Self::remove`] for call sites that build a fresh `ScopeOrder`.
    pub(crate) fn remove_exact(&mut self, order: &ScopeOrder) -> bool {
        self.scopes.remove(order)
    }

    pub(crate) fn contains(&self, order: &ScopeOrder) -> bool {
        self.scopes.contains(order)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &ScopeOrder> {
        self.scopes.iter()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    pub(crate) fn first(&self) -> Option<ScopeOrder> {
        self.scopes.iter().next().copied()
    }

    pub(crate) fn pop_first(&mut self) -> Option<ScopeOrder> {
        self.scopes.pop_first()
    }
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

impl Ord for DirtyTasks {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialOrd for DirtyTasks {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
