use super::UpdatePriority;
use crate::{ScopeId, Task, innerlude::BoxedAnyProps};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Eq)]
pub struct ScopeOrder {
    pub(crate) priority: UpdatePriority,
    pub(crate) height: u32,
    pub(crate) id: ScopeId,
}

impl ScopeOrder {
    pub fn new(height: u32, id: ScopeId) -> Self {
        Self {
            priority: UpdatePriority::Default,
            height,
            id,
        }
    }

    pub fn with_priority(height: u32, id: ScopeId, priority: UpdatePriority) -> Self {
        Self {
            priority,
            height,
            id,
        }
    }
}

impl PartialEq for ScopeOrder {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.height == other.height && self.id == other.id
    }
}

impl PartialOrd for ScopeOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScopeOrder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then(self.id.cmp(&other.id))
            .then(self.height.cmp(&other.height))
    }
}

impl Hash for ScopeOrder {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.priority.hash(state);
        self.height.hash(state);
        self.id.hash(state);
    }
}

#[derive(Debug, Default)]
pub(crate) struct DirtyFiberQueue {
    scopes: BTreeMap<ScopeId, BTreeMap<UpdatePriority, ScopeOrder>>,
}

impl DirtyFiberQueue {
    pub(crate) fn insert(&mut self, order: ScopeOrder) {
        self.scopes
            .entry(order.id)
            .or_default()
            .insert(order.priority, order);
    }

    pub(crate) fn remove(&mut self, order: &ScopeOrder) -> bool {
        self.remove_scope(order.id)
    }

    pub(crate) fn remove_scope(&mut self, id: ScopeId) -> bool {
        self.scopes.remove(&id).is_some()
    }

    pub(crate) fn contains(&self, order: &ScopeOrder) -> bool {
        self.scopes.contains_key(&order.id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &ScopeOrder> {
        self.scopes.values().flat_map(|orders| orders.values())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    pub(crate) fn first(&self) -> Option<ScopeOrder> {
        self.iter().min().copied()
    }

    pub(crate) fn pop_first(&mut self) -> Option<ScopeOrder> {
        let order = self.first()?;
        self.remove_exact(&order);
        Some(order)
    }

    pub(crate) fn remove_exact(&mut self, order: &ScopeOrder) -> bool {
        let Some(orders) = self.scopes.get_mut(&order.id) else {
            return false;
        };

        let removed = orders.remove(&order.priority).is_some();
        if orders.is_empty() {
            self.scopes.remove(&order.id);
        }
        removed
    }

    pub(crate) fn deferred_priority_for_scope(
        &self,
        id: ScopeId,
        current: UpdatePriority,
    ) -> Option<UpdatePriority> {
        self.scopes
            .get(&id)?
            .keys()
            .copied()
            .filter(|priority| *priority > current)
            .min()
    }
}

pub(crate) struct ComponentPropsDiff {
    pub(crate) priority: UpdatePriority,
    pub(crate) updates: Vec<ComponentPropsUpdate>,
}

pub(crate) struct ComponentPropsUpdate {
    pub(crate) scope: ScopeId,
    pub(crate) props: BoxedAnyProps,
}

impl Clone for ComponentPropsUpdate {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope,
            props: self.props.duplicate(),
        }
    }
}

impl std::fmt::Debug for ComponentPropsDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentPropsDiff")
            .field("priority", &self.priority)
            .field("updates", &self.updates.len())
            .finish()
    }
}

impl std::fmt::Debug for ComponentPropsUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentPropsUpdate")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
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
