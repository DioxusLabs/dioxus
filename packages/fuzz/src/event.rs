use crate::ops::EventBehaviorSpec;
use std::{cell::RefCell, rc::Rc};

pub(crate) type ListenerDriver = Rc<dyn Fn(EventBehaviorSpec)>;

#[derive(Clone)]
struct ListenerDriverState {
    behavior: EventBehaviorSpec,
    driver: Option<ListenerDriver>,
}

impl Default for ListenerDriverState {
    fn default() -> Self {
        Self {
            behavior: EventBehaviorSpec::Noop,
            driver: None,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct EventState {
    current: Rc<RefCell<ListenerDriverState>>,
}

impl EventState {
    pub(crate) fn with_listener_driver<R>(
        &self,
        behavior: EventBehaviorSpec,
        driver: ListenerDriver,
        f: impl FnOnce() -> R,
    ) -> R {
        let previous = self.current.replace(ListenerDriverState {
            behavior,
            driver: Some(driver),
        });
        let _guard = ListenerDriverGuard {
            state: self.clone(),
            previous,
        };
        f()
    }

    pub(crate) fn handle_listener_event(&self) {
        let state = self.current.borrow().clone();
        if state.behavior == EventBehaviorSpec::Noop {
            return;
        }

        if let Some(driver) = state.driver {
            driver(state.behavior);
        }
    }
}

struct ListenerDriverGuard {
    state: EventState,
    previous: ListenerDriverState,
}

impl Drop for ListenerDriverGuard {
    fn drop(&mut self) {
        self.state.current.replace(self.previous.clone());
    }
}
