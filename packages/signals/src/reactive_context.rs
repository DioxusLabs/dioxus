use dioxus_core::prelude::{
    current_scope_id, has_context, provide_context, schedule_update_any, ScopeId,
};
use futures_channel::mpsc::UnboundedReceiver;
use generational_box::SyncStorage;
use std::{cell::RefCell, hash::Hash};

use crate::{CopyValue, Writable};

/// A context for signal reads and writes to be directed to
///
/// When a signal calls .read(), it will look for the current ReactiveContext to read from.
/// If it doesn't find it, then it will try and insert a context into the nearest component scope via context api.
///
/// When the ReactiveContext drops, it will remove itself from the associated contexts attached to signal
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReactiveContext {
    inner: CopyValue<Inner, SyncStorage>,
}

thread_local! {
    static CURRENT: RefCell<Vec<ReactiveContext>> = const { RefCell::new(vec![]) };
}

impl std::fmt::Display for ReactiveContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        {
            use crate::Readable;
            if let Ok(read) = self.inner.try_read() {
                if let Some(scope) = read.scope {
                    return write!(f, "ReactiveContext(for scope: {:?})", scope);
                }
                return write!(f, "ReactiveContext created at {}", read.origin);
            }
        }
        write!(f, "ReactiveContext")
    }
}

impl ReactiveContext {
    /// Create a new reactive context
    #[track_caller]
    pub fn new() -> (Self, UnboundedReceiver<()>) {
        Self::new_with_origin(std::panic::Location::caller())
    }

    /// Create a new reactive context with a location for debugging purposes
    /// This is useful for reactive contexts created within closures
    pub fn new_with_origin(
        origin: &'static std::panic::Location<'static>,
    ) -> (Self, UnboundedReceiver<()>) {
        let (tx, rx) = futures_channel::mpsc::unbounded();
        let callback = move || {
            let _ = tx.unbounded_send(());
        };
        let _self = Self::new_with_callback(callback, current_scope_id().unwrap(), origin);
        (_self, rx)
    }

    /// Create a new reactive context that may update a scope. When any signal that this context subscribes to changes, the callback will be run
    pub fn new_with_callback(
        callback: impl FnMut() + Send + Sync + 'static,
        scope: ScopeId,
        #[allow(unused)] origin: &'static std::panic::Location<'static>,
    ) -> Self {
        let inner = Inner {
            self_: None,
            update: Box::new(callback),
            #[cfg(debug_assertions)]
            origin,
            #[cfg(debug_assertions)]
            scope: None,
        };

        let mut self_ = Self {
            inner: CopyValue::new_maybe_sync_in_scope(inner, scope),
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
        let update_any = schedule_update_any();
        let scope_id = current_scope_id().unwrap();
        let update_scope = move || {
            tracing::trace!("Marking scope {:?} as dirty", scope_id);
            update_any(scope_id)
        };

        // Otherwise, create a new context at the current scope
        #[allow(unused_mut)]
        let mut reactive_context = ReactiveContext::new_with_callback(
            update_scope,
            scope_id,
            std::panic::Location::caller(),
        );
        #[cfg(debug_assertions)]
        {
            // Associate the reactive context with the current scope for debugging
            reactive_context.inner.write().scope = Some(scope_id);
        }
        Some(provide_context(reactive_context))
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
        if let Ok(mut self_write) = self.inner.try_write_unchecked() {
            #[cfg(debug_assertions)]
            {
                tracing::trace!(
                    "Marking reactive context created at {} as dirty",
                    self_write.origin
                );
            }

            (self_write.update)();

            true
        } else {
            false
        }
    }

    /// Get the scope that inner CopyValue is associated with
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }
}

impl Hash for ReactiveContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.id().hash(state);
    }
}

struct Inner {
    self_: Option<ReactiveContext>,

    // Futures will call .changed().await
    update: Box<dyn FnMut() + Send + Sync>,

    // Debug information for signal subscriptions
    #[cfg(debug_assertions)]
    origin: &'static std::panic::Location<'static>,

    #[cfg(debug_assertions)]
    // The scope that this reactive context is associated with
    scope: Option<ScopeId>,
}
