//! Tracked and computed state in Dioxus

use dioxus_core::{ScopeId, ScopeState};
use slab::Slab;
use std::{
    cell::{RefCell, RefMut},
    collections::HashSet,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Create a new tracked state.
/// Tracked state is state that can drive Selector state
///
/// It will efficiently update any Selector state that is reading from it, but it is not readable on its own.
///
/// ```rust
/// use dioxus::prelude::*;
///
/// #[component]
/// fn Parent(cx: Scope) -> Element {
///    let count = use_tracked_state(cx, || 0);
///
///    render! {
///        Child {
///            count: count.clone(),
///        }
///    }
/// }
///
/// #[component]
/// fn Child(cx: Scope, count: Tracked<usize>) -> Element {
///    let less_than_five = use_selector(cx, count, |count| *count < 5);
///
///    render! {
///        "{less_than_five}"
///    }
/// }
/// ```
#[must_use]
pub fn use_tracked_state<T: 'static>(cx: &ScopeState, init: impl FnOnce() -> T) -> &Tracked<T> {
    cx.use_hook(|| {
        let init = init();
        Tracked::new(cx, init)
    })
}

/// Tracked state is state that can drive Selector state
///
/// Tracked state will efficiently update any Selector state that is reading from it, but it is not readable on it's own.
#[derive(Clone)]
pub struct Tracked<I> {
    state: Rc<RefCell<I>>,
    update_any: std::sync::Arc<dyn Fn(ScopeId)>,
    subscribers: SubscribedCallbacks<I>,
}

impl<I: PartialEq> PartialEq for Tracked<I> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<I> Tracked<I> {
    /// Create a new tracked state
    pub fn new(cx: &ScopeState, state: I) -> Self {
        let subscribers = std::rc::Rc::new(std::cell::RefCell::new(Slab::new()));
        Self {
            state: Rc::new(RefCell::new(state)),
            subscribers,
            update_any: cx.schedule_update_any(),
        }
    }

    /// Create a new Selector state from this tracked state
    pub fn compute<O: PartialEq + 'static>(
        &self,
        mut compute: impl FnMut(&I) -> O + 'static,
    ) -> Selector<O, I> {
        let subscribers = Rc::new(RefCell::new(HashSet::new()));
        let state = Rc::new(RefCell::new(compute(&self.state.borrow())));
        let update_any = self.update_any.clone();

        Selector {
            value: state.clone(),
            subscribers: subscribers.clone(),
            _tracker: Rc::new(self.track(move |input_state| {
                let new = compute(input_state);
                let different = {
                    let state = state.borrow();
                    *state != new
                };
                if different {
                    let mut state = state.borrow_mut();
                    *state = new;
                    for id in subscribers.borrow().iter().copied() {
                        (update_any)(id);
                    }
                }
            })),
        }
    }

    pub(crate) fn track(&self, update: impl FnMut(&I) + 'static) -> Tracker<I> {
        let mut subscribers = self.subscribers.borrow_mut();
        let id = subscribers.insert(Box::new(update));
        Tracker {
            subscribers: self.subscribers.clone(),
            id,
        }
    }

    /// Write to the tracked state
    pub fn write(&self) -> TrackedMut<'_, I> {
        TrackedMut {
            state: self.state.borrow_mut(),
            subscribers: self.subscribers.clone(),
        }
    }
}

/// A mutable reference to tracked state
pub struct TrackedMut<'a, I> {
    state: RefMut<'a, I>,
    subscribers: SubscribedCallbacks<I>,
}

impl<'a, I> Deref for TrackedMut<'a, I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<'a, I> DerefMut for TrackedMut<'a, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<'a, I> Drop for TrackedMut<'a, I> {
    fn drop(&mut self) {
        let state = self.state.deref();
        for (_, sub) in &mut *self.subscribers.borrow_mut() {
            sub(state);
        }
    }
}

type SubscribedCallbacks<I> = std::rc::Rc<std::cell::RefCell<Slab<Box<dyn FnMut(&I) + 'static>>>>;

pub(crate) struct Tracker<I> {
    subscribers: SubscribedCallbacks<I>,
    id: usize,
}

impl<I> Drop for Tracker<I> {
    fn drop(&mut self) {
        let _ = self.subscribers.borrow_mut().remove(self.id);
    }
}

#[must_use = "Consider using the `use_effect` hook to rerun an effect whenever the tracked state changes if you don't need the result of the computation"]
pub fn use_selector<I: 'static, O: Clone + PartialEq + 'static>(
    cx: &ScopeState,
    tracked: &Tracked<I>,
    init: impl FnMut(&I) -> O + 'static,
) -> O {
    let selector = cx.use_hook(|| tracked.compute(init));
    selector.use_state(cx)
}

/// Selector state is state that is derived from tracked state
///
/// Whenever the tracked state changes, the Selector state will be updated and any components reading from it will be rerun
#[derive(Clone)]
pub struct Selector<T, I> {
    _tracker: Rc<Tracker<I>>,
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<HashSet<ScopeId>>>,
}

impl<T, I> PartialEq for Selector<T, I> {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.value, &other.value)
    }
}

impl<T: Clone + PartialEq, I> Selector<T, I> {
    /// Read the Selector state and subscribe to updates
    pub fn use_state(&self, cx: &ScopeState) -> T {
        cx.use_hook(|| {
            let id = cx.scope_id();
            self.subscribers.borrow_mut().insert(id);

            ComputedRead {
                scope: cx.scope_id(),
                subscribers: self.subscribers.clone(),
            }
        });
        self.value.borrow().clone()
    }
}

struct ComputedRead {
    scope: ScopeId,
    subscribers: std::rc::Rc<std::cell::RefCell<std::collections::HashSet<ScopeId>>>,
}

impl Drop for ComputedRead {
    fn drop(&mut self) {
        self.subscribers.borrow_mut().remove(&self.scope);
    }
}
