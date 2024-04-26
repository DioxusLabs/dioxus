use crate::innerlude::ScopeOrder;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::VecDeque;

/// Effects will always run after all changes to the DOM have been applied.
///
/// Effects are the lowest priority task in the scheduler.
/// They are run after all other dirty scopes and futures have been resolved. Other dirty scopes and futures may cause the component this effect is attached to to rerun, which would update the DOM.
pub(crate) struct Effect {
    // The scope that the effect is attached to
    scope: ScopeOrder,
    // The callbacks that will be run when effects are rerun
    effect: RefCell<VecDeque<Box<dyn FnOnce() + 'static>>>,
}

impl Effect {
    pub(crate) fn new(scope: ScopeOrder, f: impl FnOnce() + 'static) -> Self {
        let mut effect = VecDeque::new();
        effect.push_back(Box::new(f) as Box<dyn FnOnce() + 'static>);
        Self {
            scope,
            effect: RefCell::new(effect),
        }
    }

    pub(crate) fn push_back(&self, f: impl FnOnce() + 'static) {
        self.effect.borrow_mut().push_back(Box::new(f));
    }

    pub fn run(&self) {
        let mut effect = self.effect.borrow_mut();
        while let Some(f) = effect.pop_front() {
            f();
        }
    }
}

impl PartialOrd for Effect {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.scope.cmp(&other.scope))
    }
}

impl Ord for Effect {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.scope.cmp(&other.scope)
    }
}

impl PartialEq for Effect {
    fn eq(&self, other: &Self) -> bool {
        self.scope == other.scope
    }
}

impl Eq for Effect {}

impl Borrow<ScopeOrder> for Effect {
    fn borrow(&self) -> &ScopeOrder {
        &self.scope
    }
}
