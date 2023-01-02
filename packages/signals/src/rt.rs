use std::{any::Any, cell::RefCell, sync::Arc};

use dioxus_core::ScopeId;
use slab::Slab;

thread_local! {
    static RUNTIMES: RefCell<Vec<&'static SignalRt>> = RefCell::new(Vec::new());
}

/// Provide the runtime for signals
///
/// This will reuse dead runtimes
pub fn claim_rt(update_any: Arc<dyn Fn(ScopeId)>) -> &'static SignalRt {
    RUNTIMES.with(|runtimes| {
        if let Some(rt) = runtimes.borrow_mut().pop() {
            return rt;
        }

        Box::leak(Box::new(SignalRt {
            signals: RefCell::new(Slab::new()),
            update_any,
        }))
    })
}

/// Push this runtime into the global runtime list
pub fn reclam_rt(_rt: &'static SignalRt) {
    RUNTIMES.with(|runtimes| {
        runtimes.borrow_mut().push(_rt);
    });
}

pub struct SignalRt {
    signals: RefCell<Slab<Inner>>,
    update_any: Arc<dyn Fn(ScopeId)>,
}

impl SignalRt {
    pub fn init<T: 'static>(&self, val: T) -> usize {
        self.signals.borrow_mut().insert(Inner {
            value: Box::new(val),
            subscribers: Vec::new(),
        })
    }

    pub fn subscribe(&self, id: usize, subscriber: ScopeId) {
        self.signals.borrow_mut()[id].subscribers.push(subscriber);
    }

    pub fn get<T: Clone + 'static>(&self, id: usize) -> T {
        self.signals.borrow()[id]
            .value
            .downcast_ref::<T>()
            .cloned()
            .unwrap()
    }

    pub fn set<T: 'static>(&self, id: usize, value: T) {
        let mut signals = self.signals.borrow_mut();
        let inner = &mut signals[id];
        inner.value = Box::new(value);

        for subscriber in inner.subscribers.iter() {
            (self.update_any)(*subscriber);
        }
    }

    pub fn remove(&self, id: usize) {
        self.signals.borrow_mut().remove(id);
    }

    pub fn with<T: 'static, O>(&self, id: usize, f: impl FnOnce(&T) -> O) -> O {
        let signals = self.signals.borrow();
        let inner = &signals[id];
        let inner = inner.value.downcast_ref::<T>().unwrap();
        f(&*inner)
    }

    pub(crate) fn read<T: 'static>(&self, id: usize) -> std::cell::Ref<T> {
        let signals = self.signals.borrow();
        std::cell::Ref::map(signals, |signals| {
            signals[id].value.downcast_ref::<T>().unwrap()
        })
    }

    pub(crate) fn write<T: 'static>(&self, id: usize) -> std::cell::RefMut<T> {
        let signals = self.signals.borrow_mut();
        std::cell::RefMut::map(signals, |signals| {
            signals[id].value.downcast_mut::<T>().unwrap()
        })
    }
}

struct Inner {
    value: Box<dyn Any>,
    subscribers: Vec<ScopeId>,
}
