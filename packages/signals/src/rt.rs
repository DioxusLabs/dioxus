use std::{any::Any, cell::RefCell, ops::Deref, rc::Rc, sync::Arc};

use dioxus_core::ScopeId;
use generational_arena::{Arena, Index};

use crate::Signal;

pub(crate) type RunTimeId = Index;

thread_local! {
    // we cannot drop these since any future might be using them
    static RUNTIMES: RefCell<Arena<&'static SignalRt>> = RefCell::new(Arena::new());
}

#[inline(always)]
pub fn with_rt<R>(idx: Index, f: impl FnOnce(&SignalRt) -> R) -> R {
    try_with_rt(idx, f).expect("Attempted to get a runtime that does not exist. This is likely from using a signal after the scope it was created in has been dropped.")
}

#[inline(always)]
pub(crate) fn try_with_rt<R>(idx: Index, f: impl FnOnce(&SignalRt) -> R) -> Option<R> {
    RUNTIMES.with(|runtimes| {
        let runtimes = runtimes.borrow();
        runtimes.get(idx).map(|rt| f(rt))
    })
}

/// Provide the runtime for signals
///
/// This will reuse dead runtimes
fn claim_rt(update_any: Arc<dyn Fn(ScopeId)>) -> RunTimeId {
    RUNTIMES.with(|runtimes| {
        runtimes.borrow_mut().insert_with(|idx| {
            Box::leak(Box::new(SignalRt {
                idx,
                signals: RefCell::new(Arena::new()),
                update_any,
            }))
        })
    })
}

pub struct RuntimeOwner {
    idx: RunTimeId,
}

impl Deref for RuntimeOwner {
    type Target = RunTimeId;

    fn deref(&self) -> &Self::Target {
        &self.idx
    }
}

impl RuntimeOwner {
    pub fn new(update_any: Arc<dyn Fn(ScopeId)>) -> Self {
        Self {
            idx: claim_rt(update_any),
        }
    }
}

impl Drop for RuntimeOwner {
    fn drop(&mut self) {
        // reclaim the runtime
        RUNTIMES.with(|runtimes| {
            let mut borrow = runtimes.borrow_mut();
            borrow.remove(self.idx);
        });
    }
}

pub struct SignalRt {
    pub(crate) idx: RunTimeId,
    pub(crate) signals: RefCell<Arena<Inner>>,
    pub(crate) update_any: Arc<dyn Fn(ScopeId)>,
}

impl std::fmt::Debug for SignalRt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalRt")
            .field("idx", &self.idx)
            .field("signals", &self.signals)
            .finish()
    }
}

impl SignalRt {
    pub fn size(&self) -> usize {
        self.signals.borrow().len()
    }

    pub fn init<T: 'static>(&self, val: T) -> Index {
        self.signals.borrow_mut().insert(Inner {
            value: Box::new(val),
            subscribers: Vec::new(),
            getter: None,
        })
    }

    pub fn subscribe(&self, id: Index, subscriber: ScopeId) {
        self.signals.borrow_mut()[id].subscribers.push(subscriber);
    }

    pub fn get<T: Clone + 'static>(&self, id: Index) -> T {
        self.signals.borrow()[id]
            .value
            .downcast_ref::<T>()
            .cloned()
            .unwrap()
    }

    pub fn set<T: 'static>(&self, id: Index, value: T) {
        let mut signals = self.signals.borrow_mut();
        let inner = &mut signals[id];
        inner.value = Box::new(value);

        for subscriber in inner.subscribers.iter() {
            (self.update_any)(*subscriber);
        }
    }

    pub fn remove<T>(&self, signal: &Signal<T>) {
        self.signals.borrow_mut().remove(signal.id);
    }

    pub fn with<T: 'static, O>(&self, id: Index, f: impl FnOnce(&T) -> O) -> O {
        let signals = self.signals.borrow();
        let inner = &signals[id];
        let inner = inner.value.downcast_ref::<T>().unwrap();
        f(inner)
    }

    pub(crate) fn update<T: 'static, O>(&self, id: Index, f: impl FnOnce(&mut T) -> O) -> O {
        let mut signals = self.signals.borrow_mut();

        let inner = &mut signals[id];
        let r = f(inner.value.downcast_mut::<T>().unwrap());

        for subscriber in inner.subscribers.iter() {
            (self.update_any)(*subscriber);
        }

        r
    }

    pub(crate) fn getter<T: 'static + Clone>(&self, id: Index) -> Rc<dyn Fn() -> T> {
        let mut signals = self.signals.borrow_mut();
        let inner = &mut signals[id];
        let idx = self.idx;
        let getter: Rc<dyn Fn()> = match &mut inner.getter {
            Some(getter) => {
                let getter: Rc<dyn Fn() -> T> = unsafe { std::mem::transmute(getter.clone()) };
                return getter;
            }
            None => {
                let r = move || with_rt(idx, |rt| rt.get::<T>(id));
                let getter: Rc<dyn Fn() -> T> = Rc::new(r);
                let getter: Rc<dyn Fn()> = unsafe { std::mem::transmute(getter) };
                inner.getter = Some(getter.clone());
                getter
            }
        };
        unsafe { std::mem::transmute(getter) }
    }
}

pub(crate) struct Inner {
    pub value: Box<dyn Any>,
    pub subscribers: Vec<ScopeId>,

    pub getter: Option<Rc<dyn Fn()>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("value", &self.value)
            .field("subscribers", &self.subscribers)
            .finish()
    }
}
