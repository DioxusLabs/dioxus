use crate::ops::EventBehaviorSpec;
use std::{cell::RefCell, rc::Rc};

type ListenerDriver = Rc<dyn Fn(EventBehaviorSpec)>;

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

thread_local! {
    static LISTENER_DRIVER: RefCell<ListenerDriverState> = RefCell::new(ListenerDriverState::default());
}

pub(crate) fn with_listener_driver<R>(
    behavior: EventBehaviorSpec,
    driver: ListenerDriver,
    f: impl FnOnce() -> R,
) -> R {
    let previous = LISTENER_DRIVER.with(|current| {
        current.replace(ListenerDriverState {
            behavior,
            driver: Some(driver),
        })
    });
    let _guard = ListenerDriverGuard { previous };
    f()
}

pub(crate) fn handle_listener_event() {
    let state = LISTENER_DRIVER.with(|current| current.borrow().clone());
    if state.behavior == EventBehaviorSpec::Noop {
        return;
    }

    if let Some(driver) = state.driver {
        driver(state.behavior);
    }
}

struct ListenerDriverGuard {
    previous: ListenerDriverState,
}

impl Drop for ListenerDriverGuard {
    fn drop(&mut self) {
        LISTENER_DRIVER.with(|current| {
            current.replace(self.previous.clone());
        });
    }
}
