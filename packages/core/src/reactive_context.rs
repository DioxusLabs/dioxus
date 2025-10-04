use crate::{current_scope_id, scope_context::Scope, tasks::SchedulerMsg, Runtime, ScopeId};
use futures_channel::mpsc::UnboundedReceiver;
use generational_box::{BorrowMutError, GenerationalBox, SyncStorage};
use std::{
    cell::RefCell,
    collections::HashSet,
    hash::Hash,
    sync::{Arc, Mutex},
};

#[doc = include_str!("../docs/reactivity.md")]
#[derive(Clone, Copy)]
pub struct ReactiveContext {
    scope: ScopeId,
    inner: GenerationalBox<Inner, SyncStorage>,
}

impl PartialEq for ReactiveContext {
    fn eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

impl Eq for ReactiveContext {}

thread_local! {
    static CURRENT: RefCell<Vec<ReactiveContext>> = const { RefCell::new(vec![]) };
}

impl std::fmt::Display for ReactiveContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        {
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
            // If there is already an update queued, we don't need to queue another
            if !tx.is_empty() {
                return;
            }
            let _ = tx.unbounded_send(());
        };
        let _self = Self::new_with_callback(callback, current_scope_id(), origin);
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
            subscribers: Default::default(),
            #[cfg(debug_assertions)]
            origin,
            #[cfg(debug_assertions)]
            scope: None,
        };

        let owner = Runtime::current().scope_owner(scope);

        let self_ = Self {
            scope,
            inner: owner.insert(inner),
        };

        self_.inner.write().self_ = Some(self_);

        self_
    }

    /// Get the current reactive context from the nearest reactive hook or scope
    pub fn current() -> Option<Self> {
        CURRENT.with(|current| current.borrow().last().cloned())
    }

    /// Create a reactive context for a scope id
    pub(crate) fn new_for_scope(scope: &Scope, runtime: &Runtime) -> Self {
        let id = scope.id;
        let sender = runtime.sender.clone();
        let update_scope = move || {
            _ = sender.unbounded_send(SchedulerMsg::Immediate(id));
        };

        // Otherwise, create a new context at the current scope
        let inner = Inner {
            self_: None,
            update: Box::new(update_scope),
            subscribers: Default::default(),
            #[cfg(debug_assertions)]
            origin: std::panic::Location::caller(),
            #[cfg(debug_assertions)]
            scope: Some(id),
        };

        let owner = scope.owner();

        let self_ = Self {
            scope: id,
            inner: owner.insert(inner),
        };

        self_.inner.write().self_ = Some(self_);

        self_
    }

    /// Clear all subscribers to this context
    pub fn clear_subscribers(&self) {
        // The key type is mutable, but the hash is stable through mutations because we hash by pointer
        #[allow(clippy::mutable_key_type)]
        let old_subscribers = std::mem::take(&mut self.inner.write().subscribers);
        for subscriber in old_subscribers {
            subscriber.0.remove(self);
        }
    }

    /// Update the subscribers
    pub(crate) fn update_subscribers(&self) {
        #[allow(clippy::mutable_key_type)]
        let subscribers = &self.inner.read().subscribers;
        for subscriber in subscribers.iter() {
            subscriber.0.add(*self);
        }
    }

    /// Reset the reactive context and then run the callback in the context. This can be used to create custom reactive hooks like `use_memo`.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_core::ReactiveContext;
    /// # use futures_util::StreamExt;
    /// fn use_simplified_memo(mut closure: impl FnMut() -> i32 + 'static) -> Signal<i32> {
    ///     use_hook(|| {
    ///         // Create a new reactive context and channel that will receive a value every time a value the reactive context subscribes to changes
    ///         let (reactive_context, mut changed) = ReactiveContext::new();
    ///         // Compute the value of the memo inside the reactive context. This will subscribe the reactive context to any values you read inside the closure
    ///         let value = reactive_context.reset_and_run_in(&mut closure);
    ///         // Create a new signal with the value of the memo
    ///         let mut signal = Signal::new(value);
    ///         // Create a task that reruns the closure when the reactive context changes
    ///         spawn(async move {
    ///             while changed.next().await.is_some() {
    ///                 // Since we reset the reactive context as we run the closure, our memo will only subscribe to the new values that are read in the closure
    ///                 let new_value = reactive_context.run_in(&mut closure);
    ///                 if new_value != value {
    ///                     signal.set(new_value);
    ///                 }
    ///             }
    ///         });
    ///         signal
    ///     })
    /// }
    ///
    /// let mut boolean = use_signal(|| false);
    /// let mut count = use_signal(|| 0);
    /// // Because we use `reset_and_run_in` instead of just `run_in`, our memo will only subscribe to the signals that are read this run of the closure (initially just the boolean)
    /// let memo = use_simplified_memo(move || if boolean() { count() } else { 0 });
    /// println!("{memo}");
    /// // Because the count signal is not read in this run of the closure, the memo will not rerun
    /// count += 1;
    /// println!("{memo}");
    /// // Because the boolean signal is read in this run of the closure, the memo will rerun
    /// boolean.toggle();
    /// println!("{memo}");
    /// // If we toggle the boolean again, and the memo unsubscribes from the count signal
    /// boolean.toggle();
    /// println!("{memo}");
    /// ```
    pub fn reset_and_run_in<O>(&self, f: impl FnOnce() -> O) -> O {
        self.clear_subscribers();
        self.run_in(f)
    }

    /// Run this function in the context of this reactive context
    ///
    /// This will set the current reactive context to this context for the duration of the function.
    /// You can then get information about the current subscriptions.
    pub fn run_in<O>(&self, f: impl FnOnce() -> O) -> O {
        CURRENT.with(|current| current.borrow_mut().push(*self));
        let out = f();
        CURRENT.with(|current| current.borrow_mut().pop());
        self.update_subscribers();
        out
    }

    /// Marks this reactive context as dirty
    ///
    /// If there's a scope associated with this context, then it will be marked as dirty too
    ///
    /// Returns true if the context was marked as dirty, or false if the context has been dropped
    pub fn mark_dirty(&self) -> bool {
        if let Ok(mut self_write) = self.inner.try_write() {
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

    /// Subscribe to this context. The reactive context will automatically remove itself from the subscriptions when it is reset.
    pub fn subscribe(&self, subscriptions: impl Into<Subscribers>) {
        match self.inner.try_write() {
            Ok(mut inner) => {
                let subscriptions = subscriptions.into();
                subscriptions.add(*self);
                inner
                    .subscribers
                    .insert(PointerHash(subscriptions.inner.clone()));
            }
            // If the context was dropped, we don't need to subscribe to it anymore
            Err(BorrowMutError::Dropped(_)) => {}
            Err(expect) => {
                panic!(
                    "Expected to be able to write to reactive context to subscribe, but it failed with: {expect:?}"
                );
            }
        }
    }

    /// Get the scope that inner CopyValue is associated with
    pub fn origin_scope(&self) -> ScopeId {
        self.scope
    }
}

impl Hash for ReactiveContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.id().hash(state);
    }
}

struct PointerHash<T: ?Sized>(Arc<T>);

impl<T: ?Sized> Hash for PointerHash<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::sync::Arc::<T>::as_ptr(&self.0).hash(state);
    }
}

impl<T: ?Sized> PartialEq for PointerHash<T> {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Eq for PointerHash<T> {}

impl<T: ?Sized> Clone for PointerHash<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct Inner {
    self_: Option<ReactiveContext>,

    // Futures will call .changed().await
    update: Box<dyn FnMut() + Send + Sync>,

    // Subscribers to this context
    subscribers: HashSet<PointerHash<dyn SubscriberList + Send + Sync>>,

    // Debug information for signal subscriptions
    #[cfg(debug_assertions)]
    origin: &'static std::panic::Location<'static>,

    #[cfg(debug_assertions)]
    // The scope that this reactive context is associated with
    scope: Option<ScopeId>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        let Some(self_) = self.self_.take() else {
            return;
        };

        for subscriber in std::mem::take(&mut self.subscribers) {
            subscriber.0.remove(&self_);
        }
    }
}

/// A list of [ReactiveContext]s that are subscribed. This is used to notify subscribers when the value changes.
#[derive(Clone)]
pub struct Subscribers {
    /// The list of subscribers.
    pub(crate) inner: Arc<dyn SubscriberList + Send + Sync>,
}

impl Default for Subscribers {
    fn default() -> Self {
        Self::new()
    }
}

impl Subscribers {
    /// Create a new no-op list of subscribers.
    pub fn new_noop() -> Self {
        struct NoopSubscribers;
        impl SubscriberList for NoopSubscribers {
            fn add(&self, _subscriber: ReactiveContext) {}

            fn remove(&self, _subscriber: &ReactiveContext) {}

            fn visit(&self, _f: &mut dyn FnMut(&ReactiveContext)) {}
        }
        Subscribers {
            inner: Arc::new(NoopSubscribers),
        }
    }

    /// Create a new list of subscribers.
    pub fn new() -> Self {
        Subscribers {
            inner: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Add a subscriber to the list.
    pub fn add(&self, subscriber: ReactiveContext) {
        self.inner.add(subscriber);
    }

    /// Remove a subscriber from the list.
    pub fn remove(&self, subscriber: &ReactiveContext) {
        self.inner.remove(subscriber);
    }

    /// Visit all subscribers in the list.
    pub fn visit(&self, mut f: impl FnMut(&ReactiveContext)) {
        self.inner.visit(&mut f);
    }
}

impl<S: SubscriberList + Send + Sync + 'static> From<Arc<S>> for Subscribers {
    fn from(inner: Arc<S>) -> Self {
        Subscribers { inner }
    }
}

/// A list of subscribers that can be notified when the value changes. This is used to track when the value changes and notify subscribers.
pub trait SubscriberList: Send + Sync {
    /// Add a subscriber to the list.
    fn add(&self, subscriber: ReactiveContext);

    /// Remove a subscriber from the list.
    fn remove(&self, subscriber: &ReactiveContext);

    /// Visit all subscribers in the list.
    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext));
}

impl SubscriberList for Mutex<HashSet<ReactiveContext>> {
    fn add(&self, subscriber: ReactiveContext) {
        if let Ok(mut lock) = self.lock() {
            lock.insert(subscriber);
        } else {
            tracing::warn!("Failed to lock subscriber list to add subscriber: {subscriber}");
        }
    }

    fn remove(&self, subscriber: &ReactiveContext) {
        if let Ok(mut lock) = self.lock() {
            lock.remove(subscriber);
        } else {
            tracing::warn!("Failed to lock subscriber list to remove subscriber: {subscriber}");
        }
    }

    fn visit(&self, f: &mut dyn FnMut(&ReactiveContext)) {
        if let Ok(lock) = self.lock() {
            lock.iter().for_each(f);
        } else {
            tracing::warn!("Failed to lock subscriber list to visit subscribers");
        }
    }
}
