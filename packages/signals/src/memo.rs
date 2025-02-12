use crate::warnings::{signal_read_and_write_in_reactive_scope, signal_write_in_component_body};
use crate::write::Writable;
use crate::{read::Readable, ReadableRef, Signal};
use crate::{read_impls, GlobalMemo};
use crate::{CopyValue, ReadOnlySignal};
use std::{
    cell::RefCell,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use dioxus_core::prelude::*;
use futures_util::StreamExt;
use generational_box::{AnyStorage, BorrowResult, UnsyncStorage};
use warnings::Warning;

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

    /// Creates a new [`GlobalMemo`] that can be used anywhere inside your dioxus app. This memo will automatically be created once per app the first time you use it.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// static SIGNAL: GlobalSignal<i32> = Signal::global(|| 0);
    /// // Create a new global memo that can be used anywhere in your app
    /// static DOUBLED: GlobalMemo<i32> = Memo::global(|| SIGNAL() * 2);
    ///
    /// fn App() -> Element {
    ///     rsx! {
    ///         button {
    ///             // When SIGNAL changes, the memo will update because the SIGNAL is read inside DOUBLED
    ///             onclick: move |_| *SIGNAL.write() += 1,
    ///             "{DOUBLED}"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// <div class="warning">
    ///
    /// Global memos are generally not recommended for use in libraries because it makes it more difficult to allow multiple instances of components you define in your library.
    ///
    /// </div>
    #[track_caller]
    pub const fn global(constructor: fn() -> T) -> GlobalMemo<T>
    where
        T: PartialEq,
    {
        GlobalMemo::new(constructor)
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
        }
        // Always mark the memo as no longer dirty even if the value didn't change
        update_write
            .dirty
            .store(false, std::sync::atomic::Ordering::Relaxed);
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
            signal_read_and_write_in_reactive_scope::allow(|| {
                signal_write_in_component_body::allow(|| self.recompute())
            });
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
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.inner.try_peek_unchecked()
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
        unsafe { Readable::deref_impl(self) }
    }
}

read_impls!(Memo<T> where T: PartialEq);

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Memo<T> {}
