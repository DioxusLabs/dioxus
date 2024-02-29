use crate::ScopeId;
use crate::Task;
use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::RefCell;
use std::hash::Hash;

#[derive(Debug, Clone, Eq)]
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

#[derive(Debug, Clone, Eq)]
pub struct DirtyScope {
    pub order: ScopeOrder,
    pub rerun_queued: Cell<bool>,
    pub tasks_queued: RefCell<Vec<Task>>,
}

impl From<ScopeOrder> for DirtyScope {
    fn from(order: ScopeOrder) -> Self {
        Self {
            order,
            rerun_queued: false.into(),
            tasks_queued: Vec::new().into(),
        }
    }
}

impl DirtyScope {
    pub fn new(height: u32, id: ScopeId) -> Self {
        ScopeOrder { height, id }.into()
    }

    pub fn queue_task(&self, task: Task) {
        self.tasks_queued.borrow_mut().push(task);
    }

    pub fn queue_rerun(&self) {
        self.rerun_queued.set(true);
    }
}

impl Borrow<ScopeOrder> for DirtyScope {
    fn borrow(&self) -> &ScopeOrder {
        &self.order
    }
}

impl PartialOrd for DirtyScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.order.cmp(&other.order))
    }
}

impl Ord for DirtyScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order.cmp(&other.order)
    }
}

impl PartialEq for DirtyScope {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

impl Hash for DirtyScope {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.order.hash(state);
    }
}
