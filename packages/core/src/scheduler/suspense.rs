use futures_util::task::ArcWake;

use super::SchedulerMsg;
use crate::ElementId;
use crate::{innerlude::Mutations, Element, ScopeId};
use std::future::Future;
use std::sync::Arc;
use std::task::Waker;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
};

/// A boundary in the VirtualDom that captures all suspended components below it
pub struct SuspenseContext {
    pub(crate) id: ScopeId,
    pub(crate) waiting_on: RefCell<HashSet<ScopeId>>,
}

impl SuspenseContext {
    /// Create a new boundary for suspense
    pub fn new(id: ScopeId) -> Self {
        Self {
            id,
            waiting_on: Default::default(),
        }
    }

    pub fn mark_suspend(&self, id: ScopeId) {
        self.waiting_on.borrow_mut().insert(id);
    }
}
