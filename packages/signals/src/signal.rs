use crate::{
    read::Readable, write::Writable, CopyValue, GlobalMemo, GlobalSignal, ReactiveContext,
    ReadOnlySignal, ReadableRef,
};
use dioxus_core::{
    prelude::{flush_sync, spawn, IntoAttributeValue},
    ScopeId,
};
use generational_box::{AnyStorage, Storage, SyncStorage, UnsyncStorage};
use std::{
    any::Any,
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

/// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// #[component]
/// fn App() -> Element {
///     let mut count = use_signal(|| 0);
///
///     // Because signals have automatic dependency tracking, if you never read them in a component, that component will not be re-rended when the signal is updated.
///     // The app component will never be rerendered in this example.
///     rsx! { Child { state: count } }
/// }
///
/// #[component]
/// fn Child(state: Signal<u32>) -> Element {
///     let state = *state;
///
///     use_future( |()| async move {
///         // Because the signal is a Copy type, we can use it in an async block without cloning it.
///         *state.write() += 1;
///     });
///
///     rsx! {
///         button {
///             onclick: move |_| *state.write() += 1,
///             "{state}"
///         }
///     }
/// }
/// ```
pub struct Signal<T: 'static, S: Storage<SignalData<T>> = UnsyncStorage> {
    pub(crate) inner: CopyValue<SignalData<T>, S>,
}

/// A signal that can safely shared between threads.
pub type SyncSignal<T> = Signal<T, SyncStorage>;

/// The data stored for tracking in a signal.
pub struct SignalData<T> {
    pub(crate) subscribers: Mutex<HashSet<ReactiveContext>>,
    pub(crate) value: T,
}

impl<T: 'static> Signal<T> {
    /// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self::new_maybe_sync(value)
    }

    /// Create a new signal with a custom owner scope. The signal will be dropped when the owner scope is dropped instead of the current scope.
    #[track_caller]
    pub fn new_in_scope(value: T, owner: ScopeId) -> Self {
        Self::new_maybe_sync_in_scope(value, owner)
    }

    /// Creates a new global Signal that can be used in a global static.
    #[track_caller]
    pub const fn global(constructor: fn() -> T) -> GlobalSignal<T> {
        GlobalSignal::new(constructor)
    }
}

impl<T: PartialEq + 'static> Signal<T> {
    /// Creates a new global Signal that can be used in a global static.
    #[track_caller]
    pub const fn global_memo(constructor: fn() -> T) -> GlobalMemo<T> {
        GlobalMemo::new(constructor)
    }

    /// Creates a new unsync Selector. The selector will be run immediately and whenever any signal it reads changes.
    ///
    /// Selectors can be used to efficiently compute derived data from signals.
    #[track_caller]
    pub fn memo(f: impl FnMut() -> T + 'static) -> ReadOnlySignal<T> {
        Self::use_maybe_sync_memo(f)
    }

    /// Creates a new Selector that may be Sync + Send. The selector will be run immediately and whenever any signal it reads changes.
    ///
    /// Selectors can be used to efficiently compute derived data from signals.
    #[track_caller]
    pub fn use_maybe_sync_memo<S: Storage<SignalData<T>>>(
        mut f: impl FnMut() -> T + 'static,
    ) -> ReadOnlySignal<T, S> {
        // Get the current reactive context
        let rc = ReactiveContext::new();

        // Create a new signal in that context, wiring up its dependencies and subscribers
        let mut state: Signal<T, S> = rc.run_in(|| Signal::new_maybe_sync(f()));

        spawn(async move {
            loop {
                flush_sync().await;
                rc.changed().await;
                let new = f();
                if new != *state.peek() {
                    *state.write() = new;
                }
            }
        });

        // And just return the readonly variant of that signal
        ReadOnlySignal::new_maybe_sync(state)
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Signal<T, S> {
    /// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
    #[track_caller]
    #[tracing::instrument(skip(value))]
    pub fn new_maybe_sync(value: T) -> Self {
        Self {
            inner: CopyValue::<SignalData<T>, S>::new_maybe_sync(SignalData {
                subscribers: Default::default(),
                value,
            }),
        }
    }

    /// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
    pub fn new_with_caller(
        value: T,
        #[cfg(debug_assertions)] caller: &'static std::panic::Location<'static>,
    ) -> Self {
        Self {
            inner: CopyValue::new_with_caller(
                SignalData {
                    subscribers: Default::default(),
                    value,
                },
                #[cfg(debug_assertions)]
                caller,
            ),
        }
    }

    /// Create a new signal with a custom owner scope. The signal will be dropped when the owner scope is dropped instead of the current scope.
    #[track_caller]
    #[tracing::instrument(skip(value))]
    pub fn new_maybe_sync_in_scope(value: T, owner: ScopeId) -> Self {
        Self {
            inner: CopyValue::<SignalData<T>, S>::new_maybe_sync_in_scope(
                SignalData {
                    subscribers: Default::default(),
                    value,
                },
                owner,
            ),
        }
    }

    /// Take the value out of the signal, invalidating the signal in the process.
    pub fn take(&self) -> T {
        self.inner.take().value
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    fn update_subscribers(&self) {
        {
            let inner = self.inner.read();

            let mut subscribers = inner.subscribers.lock().unwrap();
            subscribers.retain(|reactive_context| reactive_context.mark_dirty())
        }
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
    }
}

impl<T, S: Storage<SignalData<T>>> Readable for Signal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read(&self) -> Result<ReadableRef<Self>, generational_box::BorrowError> {
        let inner = self.inner.try_read()?;

        if let Some(reactive_context) = ReactiveContext::current() {
            inner.subscribers.lock().unwrap().insert(reactive_context);
        }

        Ok(S::map(inner, |v| &v.value))
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    fn peek(&self) -> ReadableRef<Self> {
        let inner = self.inner.read();
        S::map(inner, |v| &v.value)
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Writable for Signal<T, S> {
    type Mut<R: ?Sized + 'static> = Write<R, S>;

    fn map_mut<I: ?Sized, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Self::Mut<U> {
        Write::map(ref_, f)
    }

    fn try_map_mut<
        I: ?Sized + 'static,
        U: ?Sized + 'static,
        F: FnOnce(&mut I) -> Option<&mut U>,
    >(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Option<Self::Mut<U>> {
        Write::filter_map(ref_, f)
    }

    #[track_caller]
    fn try_write(&self) -> Result<Self::Mut<T>, generational_box::BorrowMutError> {
        self.inner.try_write().map(|inner| {
            let borrow = S::map_mut(inner, |v| &mut v.value);
            Write {
                write: borrow,
                drop_signal: Box::new(SignalSubscriberDrop { signal: *self }),
            }
        })
    }
}

impl<T> IntoAttributeValue for Signal<T>
where
    T: Clone + IntoAttributeValue,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> PartialEq for Signal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone, S: Storage<SignalData<T>> + 'static> Deref for Signal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        Readable::deref_impl(self)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize + 'static, Store: Storage<SignalData<T>>> serde::Serialize
    for Signal<T, Store>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + 'static, Store: Storage<SignalData<T>>>
    serde::Deserialize<'de> for Signal<T, Store>
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new_maybe_sync(T::deserialize(deserializer)?))
    }
}

/// A mutable reference to a signal's value.
///
/// T is the current type of the write
/// S is the storage type of the signal
pub struct Write<T: ?Sized + 'static, S: AnyStorage = UnsyncStorage> {
    write: S::Mut<T>,
    drop_signal: Box<dyn Any>,
}

impl<T: ?Sized + 'static, S: AnyStorage> Write<T, S> {
    /// Map the mutable reference to the signal's value to a new type.
    pub fn map<O: ?Sized>(myself: Self, f: impl FnOnce(&mut T) -> &mut O) -> Write<O, S> {
        let Self {
            write, drop_signal, ..
        } = myself;
        Write {
            write: S::map_mut(write, f),
            drop_signal,
        }
    }

    /// Try to map the mutable reference to the signal's value to a new type
    pub fn filter_map<O: ?Sized>(
        myself: Self,
        f: impl FnOnce(&mut T) -> Option<&mut O>,
    ) -> Option<Write<O, S>> {
        let Self {
            write, drop_signal, ..
        } = myself;
        let write = S::try_map_mut(write, f);
        write.map(|write| Write { write, drop_signal })
    }
}

impl<T: ?Sized + 'static, S: AnyStorage> Deref for Write<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T: ?Sized, S: AnyStorage> DerefMut for Write<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

struct SignalSubscriberDrop<T: 'static, S: Storage<SignalData<T>>> {
    signal: Signal<T, S>,
}

impl<T: 'static, S: Storage<SignalData<T>>> Drop for SignalSubscriberDrop<T, S> {
    fn drop(&mut self) {
        self.signal.update_subscribers();
    }
}
