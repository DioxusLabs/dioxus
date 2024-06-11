use crate::read_impls;
use crate::write::Writable;
use crate::{read::Readable, ReadableRef, Signal};
use crate::{CopyValue, ReadOnlySignal};
use std::{
    cell::RefCell,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use dioxus_core::prelude::*;
use futures_util::StreamExt;
use generational_box::{AnyStorage, UnsyncStorage};

struct UpdateInformation<T> {
    dirty: Arc<AtomicBool>,
    callback: RefCell<Box<dyn FnMut() -> T>>,
}

#[doc = include_str!("../docs/memo.md")]
#[doc(alias = "Selector")]
#[doc(alias = "UseMemo")]
#[doc(alias = "Memorize")]
pub struct Memo<T: 'static> {
    inner: Signal<T>,
    update: CopyValue<UpdateInformation<T>>,
}

impl<T> From<Memo<T>> for ReadOnlySignal<T>
where
    T: PartialEq,
{
    fn from(val: Memo<T>) -> Self {
        ReadOnlySignal::new(val.inner)
    }
}

impl<T: 'static> Memo<T> {
    /// Create a new memo
    #[track_caller]
    pub fn new(f: impl FnMut() -> T + 'static) -> Self
    where
        T: PartialEq,
    {
        Self::new_with_location(f, std::panic::Location::caller())
    }

    /// Create a new memo with an explicit location
    pub fn new_with_location(
        mut f: impl FnMut() -> T + 'static,
        location: &'static std::panic::Location<'static>,
    ) -> Self
    where
        T: PartialEq,
    {
        let dirty = Arc::new(AtomicBool::new(false));
        let (tx, mut rx) = futures_channel::mpsc::unbounded();

        let callback = {
            let dirty = dirty.clone();
            move || {
                dirty.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = tx.unbounded_send(());
            }
        };
        let rc =
            ReactiveContext::new_with_callback(callback, current_scope_id().unwrap(), location);

        // Create a new signal in that context, wiring up its dependencies and subscribers
        let mut recompute = move || rc.reset_and_run_in(&mut f);
        let value = recompute();
        let recompute = RefCell::new(Box::new(recompute) as Box<dyn FnMut() -> T>);
        let update = CopyValue::new(UpdateInformation {
            dirty,
            callback: recompute,
        });
        let state: Signal<T> = Signal::new_with_caller(value, location);

        let memo = Memo {
            inner: state,
            update,
        };

        spawn_isomorphic(async move {
            while rx.next().await.is_some() {
                // Remove any pending updates
                while rx.try_next().is_ok() {}
                memo.recompute();
            }
        });

        memo
    }

    /// Rerun the computation and update the value of the memo if the result has changed.
    #[tracing::instrument(skip(self))]
    fn recompute(&self)
    where
        T: PartialEq,
    {
        let mut update_copy = self.update;
        let update_write = update_copy.write();
        let peak = self.inner.peek();
        let new_value = (update_write.callback.borrow_mut())();
        if new_value != *peak {
            drop(peak);
            let mut copy = self.inner;
            copy.set(new_value);
            update_write
                .dirty
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
    }
}

impl<T> Readable for Memo<T>
where
    T: PartialEq,
{
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError> {
        // Read the inner generational box instead of the signal so we have more fine grained control over exactly when the subscription happens
        let read = self.inner.inner.try_read_unchecked()?;

        let needs_update = self
            .update
            .read()
            .dirty
            .swap(false, std::sync::atomic::Ordering::Relaxed);
        let result = if needs_update {
            drop(read);
            // We shouldn't be subscribed to the value here so we don't trigger the scope we are currently in to rerun even though that scope got the latest value because we synchronously update the value: https://github.com/DioxusLabs/dioxus/issues/2416
            self.recompute();
            self.inner.inner.try_read_unchecked()
        } else {
            Ok(read)
        };
        // Subscribe to the current scope before returning the value
        if let Ok(read) = &result {
            if let Some(reactive_context) = ReactiveContext::current() {
                tracing::trace!("Subscribing to the reactive context {}", reactive_context);
                reactive_context.subscribe(read.subscribers.clone());
            }
        }
        result.map(|read| <UnsyncStorage as AnyStorage>::map(read, |v| &v.value))
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn peek_unchecked(&self) -> ReadableRef<'static, Self> {
        self.inner.peek_unchecked()
    }
}

impl<T> IntoAttributeValue for Memo<T>
where
    T: Clone + IntoAttributeValue + PartialEq,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for Memo<T>
where
    T: Clone + IntoDynNode + PartialEq,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

impl<T: 'static> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Clone> Deref for Memo<T>
where
    T: PartialEq,
{
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

read_impls!(Memo<T> where T: PartialEq);

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Memo<T> {}
