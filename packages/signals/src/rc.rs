use dioxus_core::prelude::{
    consume_context, consume_context_from_scope, current_scope_id, has_context, needs_update_any,
    provide_context, schedule_update, schedule_update_any, try_consume_context, ScopeId,
};
use generational_box::{GenerationalBoxId, SyncStorage};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{cell::RefCell, hash::Hash};
use tokio::signal;

use crate::{CopyValue, RcList, Readable, Writable};

/// A context for signal reads and writes to be directed to
///
/// When a signal calls .read(), it will look for the current ReactiveContext to read from.
/// If it doesn't find it, then it will try and insert a context into the nearest component scope via context api.
///
/// When the ReactiveContext drops, it will remove itself from the the associated contexts attached to signal
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReactiveContext {
    pub inner: CopyValue<Inner, SyncStorage>,
}

thread_local! {
    static CURRENT: RefCell<Vec<ReactiveContext>> = RefCell::new(vec![]);
}

impl ReactiveContext {
    pub fn new(scope: Option<ScopeId>) -> Self {
        let (tx, rx) = flume::unbounded();

        let mut scope_subscribers = FxHashSet::default();
        if let Some(scope) = scope {
            scope_subscribers.insert(scope);
        }

        let inner = Inner {
            signal_subscribers: FxHashMap::default(),
            scope_subscribers,
            sender: tx,
            _self: None,
            receiver: rx,
        };

        let mut _self = Self {
            inner: CopyValue::new_maybe_sync(inner),
        };

        _self.inner.write()._self = Some(_self);

        _self
    }

    /// Get the current reactive context
    ///
    /// If this was set manually, then that value will be returned.
    ///
    /// If there's no current reactive context, then a new one will be created at the current scope and returned.
    pub fn current() -> Option<Self> {
        let cur = CURRENT.with(|current| current.borrow().last().cloned());

        // If we're already inside a reactive context, then return that
        if let Some(cur) = cur {
            println!("Already found context!");
            return Some(cur);
        }

        // Try and get the context out of the current scope
        let scope = current_scope_id().unwrap();

        // If we're rendering, then try and use the reactive context attached to this component

        if let Some(cx) = has_context() {
            println!("found context at {scope:?}");
            return Some(cx);
        }

        println!("creating new context at {scope:?}");

        // Otherwise, create a new context at the current scope
        Some(provide_context(ReactiveContext::new(Some(scope))))
    }

    /// Run this function in the context of this reactive context
    ///
    /// This will set the current reactive context to this context for the duration of the function.
    /// You can then get information about the current subscriptions.
    pub fn run_in<O>(&self, f: impl FnOnce() -> O) -> O {
        CURRENT.with(|current| current.borrow_mut().push(*self));
        let out = f();
        CURRENT.with(|current| current.borrow_mut().pop());
        out
    }

    /// Marks this reactive context as dirty
    ///
    /// If there's a scope associated with this context, then it will be marked as dirty too
    pub fn mark_dirty(&self) {
        for scope in self.inner.read().scope_subscribers.iter() {
            println!("marking dirty {scope:?}");
            needs_update_any(*scope);
        }

        // mark the listeners as dirty
        // If the channel is full it means that the receivers have already been marked as dirty
        _ = self.inner.read().sender.try_send(());
    }

    // Create a two-way binding between this reactive context and a signal
    pub fn link(&self, signal: GenerationalBoxId, rc_list: RcList) {
        rc_list.write().insert(*self);
        self.inner
            .write()
            .signal_subscribers
            .insert(signal, rc_list);
    }

    /// Clear all subscribers from this reactive context
    pub fn clear_subscribers(&self) {
        todo!()
    }

    /// Wait for this reactive context to change
    pub async fn changed(&self) {
        let rx = self.inner.read().receiver.clone();
        _ = rx.recv_async().await;
    }
}

impl Hash for ReactiveContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.id().hash(state);
    }
}

pub struct Inner {
    // Set of signals bound to this context
    pub signal_subscribers: FxHashMap<GenerationalBoxId, RcList>,
    scope_subscribers: FxHashSet<ScopeId>,
    _self: Option<ReactiveContext>,

    // Futures will call .changed().await
    sender: flume::Sender<()>,
    receiver: flume::Receiver<()>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Remove this context from all the subscribers
        self.signal_subscribers.values().for_each(|sub_list| {
            sub_list.write().remove(&self._self.unwrap());
        });
    }
}
