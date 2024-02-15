use dioxus_core::prelude::{
    current_scope_id, has_context, provide_context, schedule_update_any, ScopeId,
};
use generational_box::SyncStorage;
use rustc_hash::FxHashSet;
use std::{cell::RefCell, hash::Hash, sync::Arc};

use crate::{CopyValue, Readable, Writable};

/// A context for signal reads and writes to be directed to
///
/// When a signal calls .read(), it will look for the current ReactiveContext to read from.
/// If it doesn't find it, then it will try and insert a context into the nearest component scope via context api.
///
/// When the ReactiveContext drops, it will remove itself from the the associated contexts attached to signal
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReactiveContext {
    inner: CopyValue<Inner, SyncStorage>,
}

thread_local! {
    static CURRENT: RefCell<Vec<ReactiveContext>> = const { RefCell::new(vec![]) };
}

impl Default for ReactiveContext {
    fn default() -> Self {
        Self::new_for_scope(None)
    }
}

impl ReactiveContext {
    /// Create a new reactive context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new reactive context that may update a scope
    pub(crate) fn new_for_scope(scope: Option<ScopeId>) -> Self {
        let (tx, rx) = flume::unbounded();

        let mut scope_subscribers = FxHashSet::default();
        if let Some(scope) = scope {
            scope_subscribers.insert(scope);
        }

        let inner = Inner {
            scope_subscriber: scope,
            sender: tx,
            self_: None,
            update_any: schedule_update_any(),
            receiver: rx,
        };

        let mut self_ = Self {
            inner: CopyValue::new_maybe_sync_in_scope(
                inner,
                scope.or_else(current_scope_id).unwrap(),
            ),
        };

        self_.inner.write().self_ = Some(self_);

        self_
    }

    /// Get the current reactive context
    ///
    /// If this was set manually, then that value will be returned.
    ///
    /// If there's no current reactive context, then a new one will be created for the current scope and returned.
    pub fn current() -> Option<Self> {
        let cur = CURRENT.with(|current| current.borrow().last().cloned());

        // If we're already inside a reactive context, then return that
        if let Some(cur) = cur {
            return Some(cur);
        }

        // If we're rendering, then try and use the reactive context attached to this component
        if !dioxus_core::vdom_is_rendering() {
            return None;
        }
        if let Some(cx) = has_context() {
            return Some(cx);
        }

        // Otherwise, create a new context at the current scope
        Some(provide_context(ReactiveContext::new_for_scope(
            current_scope_id(),
        )))
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
    ///
    /// Returns true if the context was marked as dirty, or false if the context has been dropped
    pub fn mark_dirty(&self) -> bool {
        if let Ok(self_read) = self.inner.try_read() {
            if let Some(scope) = self_read.scope_subscriber {
                (self_read.update_any)(scope);
            }

            // mark the listeners as dirty
            // If the channel is full it means that the receivers have already been marked as dirty
            _ = self_read.sender.try_send(());
            true
        } else {
            false
        }
    }

    /// Get the scope that inner CopyValue is associated with
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
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

struct Inner {
    // A scope we mark as dirty when this context is written to
    scope_subscriber: Option<ScopeId>,
    self_: Option<ReactiveContext>,
    update_any: Arc<dyn Fn(ScopeId) + Send + Sync>,

    // Futures will call .changed().await
    sender: flume::Sender<()>,
    receiver: flume::Receiver<()>,
}
