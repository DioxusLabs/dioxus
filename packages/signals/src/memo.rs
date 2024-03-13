use crate::write::Writable;
use crate::{read::Readable, ReactiveContext, ReadableRef, Signal};
use crate::{CopyValue, Dependency, ReadOnlySignal};
use std::rc::Rc;
use std::{
    cell::RefCell,
    ops::Deref,
    panic::Location,
    sync::{atomic::AtomicBool, Arc},
};

use dioxus_core::prelude::*;
use futures_util::StreamExt;
use generational_box::UnsyncStorage;
use once_cell::sync::OnceCell;

/// A thread local that can only be read from the thread it was created on.
pub struct ThreadLocal<T> {
    value: T,
    owner: std::thread::ThreadId,
}

impl<T> ThreadLocal<T> {
    /// Create a new thread local.
    pub fn new(value: T) -> Self {
        ThreadLocal {
            value,
            owner: std::thread::current().id(),
        }
    }

    /// Get the value of the thread local.
    pub fn get(&self) -> Option<&T> {
        (self.owner == std::thread::current().id()).then_some(&self.value)
    }
}

// SAFETY: This is safe because the thread local can only be read from the thread it was created on.
unsafe impl<T> Send for ThreadLocal<T> {}
unsafe impl<T> Sync for ThreadLocal<T> {}

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
    pub fn new(mut f: impl FnMut() -> T + 'static) -> Self
    where
        T: PartialEq,
    {
        let dirty = Arc::new(AtomicBool::new(true));
        let (tx, mut rx) = futures_channel::mpsc::unbounded();

        let myself: Rc<OnceCell<Memo<T>>> = Rc::new(OnceCell::new());
        let thread_local = ThreadLocal::new(myself.clone());

        let callback = {
            let dirty = dirty.clone();
            move || match thread_local.get() {
                Some(memo) => match memo.get() {
                    Some(memo) => {
                        memo.recompute();
                    }
                    None => {
                        tracing::error!("Memo was not initialized in the same thread it was created in. This is likely a bug in dioxus");
                        dirty.store(true, std::sync::atomic::Ordering::Relaxed);
                        let _ = tx.unbounded_send(());
                    }
                },
                None => {
                    dirty.store(true, std::sync::atomic::Ordering::Relaxed);
                    let _ = tx.unbounded_send(());
                }
            }
        };
        let rc = ReactiveContext::new_with_callback(
            callback,
            current_scope_id().unwrap(),
            Location::caller(),
        );

        // Create a new signal in that context, wiring up its dependencies and subscribers
        let mut recompute = move || rc.run_in(&mut f);
        let value = recompute();
        let recompute = RefCell::new(Box::new(recompute) as Box<dyn FnMut() -> T>);
        let update = CopyValue::new(UpdateInformation {
            dirty,
            callback: recompute,
        });
        let state: Signal<T> = Signal::new(value);

        let memo = Memo {
            inner: state,
            update,
        };
        let _ = myself.set(memo);

        spawn(async move {
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

    /// Adds an explicit dependency to the memo. If the dependency changes, the memo will rerun.
    ///
    /// Signals will automatically be added as dependencies, so you don't need to call this method for them.
    ///
    /// NOTE: You must follow the rules of hooks when calling this method.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// # async fn sleep(delay: u32) {}
    ///
    /// #[component]
    /// fn Comp(count: u32) -> Element {
    ///     // Because the resource subscribes to `delay` by adding it as a dependency, the memo will rerun every time `count` changes.
    ///     let new_count = use_memo(move || async move {
    ///         count + 1
    ///     })
    ///     .use_dependencies((&count,));
    ///
    ///     todo!()
    /// }
    /// ```
    pub fn use_dependencies(self, dependency: impl Dependency) -> Self
    where
        T: PartialEq,
    {
        let mut dependencies_signal = use_hook(|| CopyValue::new(dependency.out()));
        let changed = { dependency.changed(&*dependencies_signal.read()) };
        if changed {
            dependencies_signal.set(dependency.out());
            self.recompute();
        }
        self
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
