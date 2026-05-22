use crate::{
    event::EventState, lifecycle::LifecycleState, model::Model, ops::SuspenseReadyRegistry,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone)]
pub(crate) struct HarnessContext {
    pub(crate) model: Rc<RefCell<Model>>,
    pub(crate) suspense_ready: Rc<RefCell<SuspenseReadyRegistry>>,
    pub(crate) register_suspense_ready_wakers: Rc<Cell<bool>>,
    pub(crate) events: EventState,
    pub(crate) lifecycle: LifecycleState,
}

impl Default for HarnessContext {
    fn default() -> Self {
        Self {
            model: Rc::new(RefCell::new(Model::initial())),
            suspense_ready: Rc::new(RefCell::new(SuspenseReadyRegistry::default())),
            register_suspense_ready_wakers: Rc::new(Cell::new(true)),
            events: EventState::default(),
            lifecycle: LifecycleState::default(),
        }
    }
}

impl PartialEq for HarnessContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}

impl HarnessContext {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
