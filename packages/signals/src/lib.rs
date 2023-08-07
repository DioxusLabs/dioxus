use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
    sync::Arc,
};

mod rt;
pub use rt::*;
mod effect;
pub use effect::*;
#[macro_use]
mod impls;

use dioxus_core::{
    prelude::{current_scope_id, has_context, provide_context, schedule_update_any},
    ScopeId, ScopeState,
};

pub fn use_signal<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> Signal<T> {
    *cx.use_hook(|| Signal::new(f()))
}

#[derive(Clone)]
struct Unsubscriber {
    scope: ScopeId,
    subscribers: Rc<RefCell<Vec<Rc<RefCell<Vec<ScopeId>>>>>>,
}

impl Drop for Unsubscriber {
    fn drop(&mut self) {
        for subscribers in self.subscribers.borrow().iter() {
            subscribers.borrow_mut().retain(|s| *s != self.scope);
        }
    }
}

fn current_unsubscriber() -> Unsubscriber {
    match has_context() {
        Some(rt) => rt,
        None => {
            let owner = Unsubscriber {
                scope: current_scope_id().expect("in a virtual dom"),
                subscribers: Default::default(),
            };
            provide_context(owner).expect("in a virtual dom")
        }
    }
}

struct SignalData<T> {
    subscribers: Rc<RefCell<Vec<ScopeId>>>,
    effect_subscribers: Rc<RefCell<Vec<Effect>>>,
    update_any: Arc<dyn Fn(ScopeId)>,
    value: T,
}

pub struct Signal<T: 'static> {
    inner: CopyValue<SignalData<T>>,
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: CopyValue::new(SignalData {
                subscribers: Default::default(),
                effect_subscribers: Default::default(),
                update_any: schedule_update_any().expect("in a virtual dom"),
                value,
            }),
        }
    }

    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    pub fn read(&self) -> Ref<T> {
        let inner = self.inner.read();
        if let Some(current_scope_id) = current_scope_id() {
            log::trace!(
                "{:?} subscribed to {:?}",
                self.inner.value,
                current_scope_id
            );
            let mut subscribers = inner.subscribers.borrow_mut();
            if !subscribers.contains(&current_scope_id) {
                subscribers.push(current_scope_id);
                drop(subscribers);
                let unsubscriber = current_unsubscriber();
                inner.subscribers.borrow_mut().push(unsubscriber.scope);
            }
        }
        if let Some(effect) = Effect::current() {
            let mut effect_subscribers = inner.effect_subscribers.borrow_mut();
            if !effect_subscribers.contains(&effect) {
                effect_subscribers.push(effect);
            }
        }
        Ref::map(inner, |v| &v.value)
    }

    pub fn write(&self) -> RefMut<T> {
        {
            let inner = self.inner.read();
            for &scope_id in &*inner.subscribers.borrow() {
                log::trace!(
                    "Write on {:?} triggered update on {:?}",
                    self.inner.value,
                    scope_id
                );
                (inner.update_any)(scope_id);
            }
        }

        let subscribers =
            { std::mem::take(&mut *self.inner.read().effect_subscribers.borrow_mut()) };
        for effect in subscribers {
            effect.try_run();
        }

        let inner = self.inner.write();
        RefMut::map(inner, |v| &mut v.value)
    }

    pub fn set(&mut self, value: T) {
        *self.write() = value;
    }

    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl<T: 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
