use crate::{
    event::EventState,
    lifecycle::LifecycleState,
    model::{Model, SuspenseReadyKey, select},
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[derive(Clone)]
pub(crate) struct HarnessContext {
    pub(crate) model: Rc<RefCell<Model>>,
    suspense_ready: Rc<RefCell<SuspenseReadyRegistry>>,
    register_suspense_ready_wakers: Rc<Cell<bool>>,
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

    pub(crate) fn read_model(&self) -> Model {
        self.model.borrow().clone()
    }

    pub(crate) fn with_model<R>(&self, f: impl FnOnce(&mut Model) -> R) -> R {
        f(&mut self.model.borrow_mut())
    }

    fn suspense_ready_released(&self, key: SuspenseReadyKey, required_wakes: usize) -> bool {
        self.register_suspense_ready_wakers.get()
            && self.suspense_ready.borrow().released(key, required_wakes)
    }

    fn register_suspense_ready_waker(&self, key: SuspenseReadyKey, waker: Waker) {
        if self.register_suspense_ready_wakers.get() {
            self.suspense_ready.borrow_mut().register_waker(key, waker);
        }
    }

    pub(crate) fn release_suspense_ready_task(&self, key: SuspenseReadyKey) {
        self.suspense_ready.borrow_mut().release(key);
    }

    pub(crate) fn selected_registered_ready_suspense_key(
        &self,
        selector: u8,
    ) -> Option<SuspenseReadyKey> {
        let registered = self.suspense_ready.borrow().registered_keys();

        let mut ready = Vec::new();
        self.read_model()
            .root
            .collect_ready_suspense_keys(&mut ready);
        ready.retain(|key| registered.contains(key));
        select(ready, selector)
    }

    pub(crate) fn clear_suspense_ready_tasks(&self) {
        self.suspense_ready.borrow_mut().clear();
    }

    pub(crate) fn without_suspense_ready_registration<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = self.register_suspense_ready_wakers.replace(false);
        let _guard = SuspenseReadyRegistrationGuard {
            context: self.clone(),
            previous,
        };
        f()
    }
}

/// Tracks which ready-suspense tasks have been woken (and how often), plus
/// the wakers registered by in-flight [`SuspenseReadyFuture`]s.
#[derive(Default)]
struct SuspenseReadyRegistry {
    wake_counts: Vec<(SuspenseReadyKey, usize)>,
    wakers: Vec<(SuspenseReadyKey, Waker)>,
}

impl SuspenseReadyRegistry {
    fn wake_count(&self, key: SuspenseReadyKey) -> usize {
        self.wake_counts
            .iter()
            .find_map(|(wake_key, count)| (*wake_key == key).then_some(*count))
            .unwrap_or(0)
    }

    fn released(&self, key: SuspenseReadyKey, required_wakes: usize) -> bool {
        self.wake_count(key) >= required_wakes
    }

    fn register_waker(&mut self, key: SuspenseReadyKey, waker: Waker) {
        if let Some((_, existing)) = self
            .wakers
            .iter_mut()
            .find(|(wake_key, existing)| *wake_key == key && existing.will_wake(&waker))
        {
            *existing = waker;
        } else {
            self.wakers.push((key, waker));
        }
    }

    fn release(&mut self, key: SuspenseReadyKey) {
        if let Some((_, count)) = self
            .wake_counts
            .iter_mut()
            .find(|(wake_key, _)| *wake_key == key)
        {
            *count = count.saturating_add(1);
        } else {
            self.wake_counts.push((key, 1));
        }

        for (_, waker) in self.wakers.iter().filter(|(wake_key, _)| *wake_key == key) {
            waker.wake_by_ref();
        }
    }

    fn registered_keys(&self) -> Vec<SuspenseReadyKey> {
        let mut keys = Vec::new();
        for (key, _) in &self.wakers {
            if !keys.contains(key) {
                keys.push(*key);
            }
        }
        keys
    }

    fn clear(&mut self) {
        self.wake_counts.clear();
        self.wakers.clear();
    }
}

struct SuspenseReadyRegistrationGuard {
    context: HarnessContext,
    previous: bool,
}

impl Drop for SuspenseReadyRegistrationGuard {
    fn drop(&mut self) {
        self.context
            .register_suspense_ready_wakers
            .set(self.previous);
    }
}

/// Resolves once the ready-suspense task identified by `key` has been woken
/// `required_wakes` times via [`HarnessContext::release_suspense_ready_task`].
pub(crate) struct SuspenseReadyFuture {
    pub(crate) context: HarnessContext,
    pub(crate) key: SuspenseReadyKey,
    pub(crate) required_wakes: usize,
}

impl Future for SuspenseReadyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let key = self.key;
        if self
            .context
            .suspense_ready_released(key, self.required_wakes)
        {
            Poll::Ready(())
        } else {
            self.context
                .register_suspense_ready_waker(key, cx.waker().clone());
            Poll::Pending
        }
    }
}
