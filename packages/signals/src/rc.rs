use dioxus_core::prelude::ScopeId;
use generational_box::{GenerationalBoxId, SyncStorage};
use rustc_hash::FxHashSet;
use std::{cell::RefCell, hash::Hash};

use crate::{CopyValue, Readable};

/// A context for signal reads and writes to be directed to
///
/// When a signal calls .read(), it will look for the current ReactiveContext to read from.
/// If it doesn't find it, then it will try and insert a context into the nearest component scope via context api.
///
/// When the ReactiveContext drops, it will remove itself from the the associated contexts attached to signal
#[derive(Clone, Copy, PartialEq)]
pub struct ReactiveContext {
    // todo: we dont need to use syncstorage per say
    inner: CopyValue<Inner, SyncStorage>,
}

thread_local! {
    static CURRENT: RefCell<Vec<ReactiveContext>> = RefCell::new(vec![]);
}

impl ReactiveContext {
    /// Get the current reactive context
    ///
    /// If this was set manually, then that value will be returned.
    ///
    /// If there's no current reactive context, then a new one will be created at the current scope and returned.
    pub fn current() -> Self {
        todo!()
    }

    /// Run this function in the context of this reactive context
    ///
    /// This will set the current reactive context to this context for the duration of the function.
    /// You can then get information about the current subscriptions.
    pub fn run_in(&self, f: impl FnOnce()) {
        todo!()
    }

    /// Marks this reactive context as dirty
    ///
    /// If there's a scope associated with this context, then it will be marked as dirty too
    pub fn mark_dirty(&self) {}

    /// Clear all subscribers from this reactive context
    pub fn clear_subscribers(&self) {
        todo!()
    }

    /// Wait for this reactive context to change
    pub async fn changed(&self) {
        let waiter = self.inner.read().waiters.1.clone();

        _ = waiter.recv_async().await;
    }
}

impl Hash for ReactiveContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.id().hash(state);
    }
}

struct Inner {
    // Set of signals bound to this context
    subscribers: FxHashSet<GenerationalBoxId>,

    // The scope that this context is associated with
    // This is only relevant when RC is being used to call update on a signal
    scope: Option<ScopeId>,

    // Futures will call .changed().await
    waiters: (flume::Sender<()>, flume::Receiver<()>),
}

impl Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        todo!()
    }
}
