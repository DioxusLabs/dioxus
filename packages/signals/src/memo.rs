use crate::read_impls;
use crate::write::Writable;
use crate::{read::Readable, ReactiveContext, ReadableRef, Signal};
use crate::{CopyValue, ReadOnlySignal};
use std::{
    cell::RefCell,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use dioxus_core::prelude::*;
use futures_util::StreamExt;
use generational_box::UnsyncStorage;

struct UpdateInformation<T> {
    dirty: Arc<AtomicBool>,
    callback: RefCell<Box<dyn FnMut() -> T>>,
}

/// A value that is memoized. This is useful for caching the result of a computation.
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
        let dirty = Arc::new(AtomicBool::new(true));
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
        let mut recompute = move || rc.run_in(&mut f);
        let value = recompute();
        let recompute = RefCell::new(Box::new(recompute) as Box<dyn FnMut() -> T>);
        let update = CopyValue::new(UpdateInformation {
            dirty,
            callback: recompute,
        });
        let state: Signal<T> = Signal::new_with_caller(
            value,
            #[cfg(debug_assertions)]
            location
        );

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
        let read = self.inner.try_read_unchecked();
        match read {
            Ok(r) => {
                let needs_update = self
                    .update
                    .read()
                    .dirty
                    .swap(false, std::sync::atomic::Ordering::Relaxed);
                if needs_update {
                    drop(r);
                    self.recompute();
                    self.inner.try_read_unchecked()
                } else {
                    Ok(r)
                }
            }
            Err(e) => Err(e),
        }
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
