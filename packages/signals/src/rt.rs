use std::{any::Any, cell::RefCell, sync::Arc};

use dioxus_core::ScopeId;
use slab::Slab;

thread_local! {
    // we cannot drop these since any future might be using them
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
    pub(crate) signals: RefCell<Slab<Inner>>,
    pub(crate) update_any: Arc<dyn Fn(ScopeId)>,
}

impl SignalRt {
    pub fn init<T: 'static>(&'static self, val: T) -> usize {
        self.signals.borrow_mut().insert(Inner {
            value: Box::new(val),
            subscribers: Vec::new(),
            getter: None,
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
        f(inner)
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

    pub(crate) fn getter<T: 'static + Clone>(&self, id: usize) -> &dyn Fn() -> T {
        let mut signals = self.signals.borrow_mut();
        let inner = &mut signals[id];
        let r = inner.getter.as_mut();

        if r.is_none() {
            let rt = self;
            let r = move || rt.get::<T>(id);
            let getter: Box<dyn Fn() -> T> = Box::new(r);
            let getter: Box<dyn Fn()> = unsafe { std::mem::transmute(getter) };

            inner.getter = Some(getter);
        }

        let r = inner.getter.as_ref().unwrap();

        unsafe { std::mem::transmute::<&dyn Fn(), &dyn Fn() -> T>(r) }
    }
}

pub(crate) struct Inner {
    pub value: Box<dyn Any>,
    pub subscribers: Vec<ScopeId>,

    // todo: this has a soundness hole in it that you might not run into
    pub getter: Option<Box<dyn Fn()>>,
}
