use dioxus_core::{CapturedError, RenderError, Result};
// use cr::Resource;
use dioxus_signals::{
    read_impls, CopyValue, ReadSignal, Readable, ReadableExt, ReadableRef, Signal, WritableExt,
};
use std::{marker::PhantomData, prelude::rust_2024::Future};

/// A hook to create a resource that loads data asynchronously.
///
/// To bubble errors and pending, simply use `?` on the result of the resource read.
///
/// To inspect the state of the resource, you can use the RenderError enum along with the RenderResultExt trait.
pub fn use_loader<
    F: Future<Output = Result<T, E>>,
    T: 'static,
    // T: 'static + PartialEq,
    E: Into<dioxus_core::Error>,
>(
    // pub fn use_loader<F: Future<Output = Result<T, E>>, T: 'static, E: Into<anyhow::Error>>(
    f: impl FnMut() -> F,
) -> Result<Loader<T>, Loading> {
    todo!()
}

#[derive(PartialEq)]
pub enum Loading {
    Pending(LoaderHandle<()>),

    Failed(LoaderHandle<RenderError>),
}

impl std::fmt::Debug for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading::Pending"),
            Loading::Failed(_) => write!(f, "Loading::Failed"),
        }
    }
}

impl std::fmt::Display for Loading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Loading::Pending(_) => write!(f, "Loading is still pending"),
            Loading::Failed(_) => write!(f, "Loading has failed"),
        }
    }
}

impl From<Loading> for RenderError {
    fn from(val: Loading) -> Self {
        todo!()
    }
}

#[derive(PartialEq)]
pub struct LoaderHandle<T> {
    _t: PhantomData<*const T>,
}
impl<T> LoaderHandle<T> {
    pub fn restart(&self) {
        todo!()
    }
}
impl<T> Clone for LoaderHandle<T> {
    fn clone(&self) -> Self {
        todo!()
    }
}
impl<T> Copy for LoaderHandle<T> {}

use std::{
    cell::RefCell,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use dioxus_core::{
    current_scope_id, spawn_isomorphic, IntoAttributeValue, IntoDynNode, ReactiveContext, ScopeId,
    Subscribers,
};
use futures_util::StreamExt;
use generational_box::{AnyStorage, BorrowResult, UnsyncStorage};

pub struct Loader<T> {
    inner: Signal<T>,
    update: CopyValue<UpdateInformation<T>>,
}

struct UpdateInformation<T> {
    dirty: Arc<AtomicBool>,
    callback: RefCell<Box<dyn FnMut() -> T>>,
}

impl<T> Loader<T> {
    /// Create a new memo
    #[track_caller]
    pub fn new(f: impl FnMut() -> T + 'static) -> Self
    where
        T: PartialEq + 'static,
    {
        Self::new_with_location(f, std::panic::Location::caller())
    }

    /// Create a new memo with an explicit location
    pub fn new_with_location(
        mut f: impl FnMut() -> T + 'static,
        location: &'static std::panic::Location<'static>,
    ) -> Self
    where
        T: PartialEq + 'static,
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

        let memo = Loader {
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

    // /// Creates a new [`GlobalMemo`] that can be used anywhere inside your dioxus app. This memo will automatically be created once per app the first time you use it.
    // ///
    // /// # Example
    // /// ```rust, no_run
    // /// # use dioxus::prelude::*;
    // /// static SIGNAL: GlobalSignal<i32> = Signal::global(|| 0);
    // /// // Create a new global memo that can be used anywhere in your app
    // /// static DOUBLED: GlobalMemo<i32> = Memo::global(|| SIGNAL() * 2);
    // ///
    // /// fn App() -> Element {
    // ///     rsx! {
    // ///         button {
    // ///             // When SIGNAL changes, the memo will update because the SIGNAL is read inside DOUBLED
    // ///             onclick: move |_| *SIGNAL.write() += 1,
    // ///             "{DOUBLED}"
    // ///         }
    // ///     }
    // /// }
    // /// ```
    // ///
    // /// <div class="warning">
    // ///
    // /// Global memos are generally not recommended for use in libraries because it makes it more difficult to allow multiple instances of components you define in your library.
    // ///
    // /// </div>
    // #[track_caller]
    // pub const fn global(constructor: fn() -> T) -> GlobalLoader<T>
    // where
    //     T: PartialEq + 'static,
    // {
    //     GlobalMemo::new(constructor)
    // }

    /// Restart the loader
    pub fn restart(&mut self) {
        todo!()
    }

    /// Rerun the computation and update the value of the memo if the result has changed.
    #[tracing::instrument(skip(self))]
    fn recompute(&self)
    where
        T: PartialEq + 'static,
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
    pub fn origin_scope(&self) -> ScopeId
    where
        T: 'static,
    {
        self.inner.origin_scope()
    }

    /// Get the id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId
    where
        T: 'static,
    {
        self.inner.id()
    }
}

impl<T> Readable for Loader<T>
// where
//     T: PartialEq,
{
    type Target = T;
    type Storage = UnsyncStorage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        T: 'static,
    {
        todo!()
        // // Read the inner generational box instead of the signal so we have more fine grained control over exactly when the subscription happens
        // let read = self.inner.try_read_unchecked()?;

        // let needs_update = self
        //     .update
        //     .read()
        //     .dirty
        //     .swap(false, std::sync::atomic::Ordering::Relaxed);
        // let result = if needs_update {
        //     drop(read);
        //     // We shouldn't be subscribed to the value here so we don't trigger the scope we are currently in to rerun even though that scope got the latest value because we synchronously update the value: https://github.com/DioxusLabs/dioxus/issues/2416
        //     // self.recompute();
        //     todo!();
        //     self.inner.try_read_unchecked()
        // } else {
        //     Ok(read)
        // };

        // // Subscribe to the current scope before returning the value
        // if let Ok(read) = &result {
        //     if let Some(reactive_context) = ReactiveContext::current() {
        //         tracing::trace!("Subscribing to the reactive context {}", reactive_context);
        //         reactive_context.subscribe(read.subscribers.clone());
        //     }
        // }

        // result.map(|read| <UnsyncStorage as AnyStorage>::map(read, |v| &v.value))
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        T: 'static,
    {
        self.inner.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        T: 'static,
    {
        self.inner.subscribers()
    }
}

impl<T> IntoAttributeValue for Loader<T>
where
    T: Clone + IntoAttributeValue + PartialEq + 'static,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T> IntoDynNode for Loader<T>
where
    T: Clone + IntoDynNode + PartialEq + 'static,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

impl<T: 'static> PartialEq for Loader<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Clone> Deref for Loader<T>
where
    T: PartialEq + 'static,
{
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

read_impls!(Loader<T> where T: PartialEq);

impl<T> Clone for Loader<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Loader<T> {}
