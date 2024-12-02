use crate::{default_impl, fmt_impls, write_impls, Global};
use crate::{read::*, write::*, CopyValue, GlobalMemo, GlobalSignal, ReadableRef};
use crate::{Memo, WritableRef};
use dioxus_core::prelude::*;
use generational_box::{AnyStorage, BorrowResult, Storage, SyncStorage, UnsyncStorage};
use std::sync::Arc;
use std::{
    any::Any,
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

#[doc = include_str!("../docs/signals.md")]
#[doc(alias = "State")]
#[doc(alias = "UseState")]
#[doc(alias = "UseRef")]
pub struct Signal<T: 'static, S: Storage<SignalData<T>> = UnsyncStorage> {
    pub(crate) inner: CopyValue<SignalData<T>, S>,
}

/// A signal that can safely shared between threads.
#[doc(alias = "SendSignal")]
#[doc(alias = "UseRwLock")]
#[doc(alias = "UseRw")]
#[doc(alias = "UseMutex")]
pub type SyncSignal<T> = Signal<T, SyncStorage>;

/// The data stored for tracking in a signal.
pub struct SignalData<T> {
    pub(crate) subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
    pub(crate) value: T,
}

impl<T: 'static> Signal<T> {
    /// Creates a new [`Signal`]. Signals are a Copy state management solution with automatic dependency tracking.
    ///
    /// <div class="warning">
    ///
    /// This function should generally only be called inside hooks. The signal that this function creates is owned by the current component and will only be dropped when the component is dropped. If you call this function outside of a hook many times, you will leak memory until the component is dropped.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn MyComponent() {
    ///     // ❌ Every time MyComponent runs, it will create a new signal that is only dropped when MyComponent is dropped
    ///     let signal = Signal::new(0);
    ///     use_context_provider(|| signal);
    ///     // ✅ Since the use_context_provider hook only runs when the component is created, the signal will only be created once and it will be dropped when MyComponent is dropped
    ///     let signal = use_context_provider(|| Signal::new(0));
    /// }
    /// ```
    ///
    /// </div>
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self::new_maybe_sync(value)
    }

    /// Create a new signal with a custom owner scope. The signal will be dropped when the owner scope is dropped instead of the current scope.
    #[track_caller]
    pub fn new_in_scope(value: T, owner: ScopeId) -> Self {
        Self::new_maybe_sync_in_scope(value, owner)
    }

    /// Creates a new [`GlobalSignal`] that can be used anywhere inside your dioxus app. This signal will automatically be created once per app the first time you use it.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// // Create a new global signal that can be used anywhere in your app
    /// static SIGNAL: GlobalSignal<i32> = Signal::global(|| 0);
    ///
    /// fn App() -> Element {
    ///     rsx! {
    ///         button {
    ///             onclick: move |_| *SIGNAL.write() += 1,
    ///             "{SIGNAL}"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// <div class="warning">
    ///
    /// Global signals are generally not recommended for use in libraries because it makes it more difficult to allow multiple instances of components you define in your library.
    ///
    /// </div>
    #[track_caller]
    pub const fn global(constructor: fn() -> T) -> GlobalSignal<T> {
        Global::new(constructor)
    }
}

impl<T: PartialEq + 'static> Signal<T> {
    /// Creates a new [`GlobalMemo`] that can be used anywhere inside your dioxus app. This memo will automatically be created once per app the first time you use it.
    ///
    /// # Example
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// static SIGNAL: GlobalSignal<i32> = Signal::global(|| 0);
    /// // Create a new global memo that can be used anywhere in your app
    /// static DOUBLED: GlobalMemo<i32> = Signal::global_memo(|| SIGNAL() * 2);
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
    pub const fn global_memo(constructor: fn() -> T) -> GlobalMemo<T>
    where
        T: PartialEq,
    {
        GlobalMemo::new(constructor)
    }

    /// Creates a new unsync Selector. The selector will be run immediately and whenever any signal it reads changes.
    ///
    /// Selectors can be used to efficiently compute derived data from signals.
    #[track_caller]
    pub fn memo(f: impl FnMut() -> T + 'static) -> Memo<T> {
        Memo::new(f)
    }

    /// Creates a new unsync Selector with an explicit location. The selector will be run immediately and whenever any signal it reads changes.
    ///
    /// Selectors can be used to efficiently compute derived data from signals.
    pub fn memo_with_location(
        f: impl FnMut() -> T + 'static,
        location: &'static std::panic::Location<'static>,
    ) -> Memo<T> {
        Memo::new_with_location(f, location)
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

    /// Creates a new Signal with an explicit caller. Signals are a Copy state management solution with automatic dependency tracking.
    ///
    /// This method can be used to provide the correct caller information for signals that are created in closures:
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// #[track_caller]
    /// fn use_my_signal(function: impl FnOnce() -> i32) -> Signal<i32> {
    ///     // We capture the caller information outside of the closure so that it points to the caller of use_my_custom_hook instead of the closure
    ///     let caller = std::panic::Location::caller();
    ///     use_hook(move || Signal::new_with_caller(function(), caller))
    /// }
    /// ```
    pub fn new_with_caller(value: T, caller: &'static std::panic::Location<'static>) -> Self {
        Self {
            inner: CopyValue::new_with_caller(
                SignalData {
                    subscribers: Default::default(),
                    value,
                },
                caller,
            ),
        }
    }

    /// Create a new Signal without an owner. This will leak memory if you don't manually drop it.
    pub fn leak_with_caller(value: T, caller: &'static std::panic::Location<'static>) -> Self {
        Self {
            inner: CopyValue::leak_with_caller(
                SignalData {
                    subscribers: Default::default(),
                    value,
                },
                caller,
            ),
        }
    }

    /// Create a new signal with a custom owner scope. The signal will be dropped when the owner scope is dropped instead of the current scope.
    #[track_caller]
    #[tracing::instrument(skip(value))]
    pub fn new_maybe_sync_in_scope(value: T, owner: ScopeId) -> Self {
        Self::new_maybe_sync_in_scope_with_caller(value, owner, std::panic::Location::caller())
    }

    /// Create a new signal with a custom owner scope and a custom caller. The signal will be dropped when the owner scope is dropped instead of the current scope.
    #[tracing::instrument(skip(value))]
    pub fn new_maybe_sync_in_scope_with_caller(
        value: T,
        owner: ScopeId,
        caller: &'static std::panic::Location<'static>,
    ) -> Self {
        Self {
            inner: CopyValue::<SignalData<T>, S>::new_maybe_sync_in_scope_with_caller(
                SignalData {
                    subscribers: Default::default(),
                    value,
                },
                owner,
                caller,
            ),
        }
    }

    /// Point to another signal. This will subscribe the other signal to all subscribers of this signal.
    pub fn point_to(&self, other: Self) -> BorrowResult {
        #[allow(clippy::mutable_key_type)]
        let this_subscribers = self.inner.value.read().subscribers.lock().unwrap().clone();
        let other_read = other.inner.value.read();
        for subscriber in this_subscribers.iter() {
            subscriber.subscribe(other_read.subscribers.clone());
        }
        self.inner.point_to(other.inner)
    }

    /// Drop the value out of the signal, invalidating the signal in the process.
    pub fn manually_drop(&self) {
        self.inner.manually_drop()
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    fn update_subscribers(&self) {
        {
            let inner = self.inner.read();

            // We cannot hold the subscribers lock while calling mark_dirty, because mark_dirty can run user code which may cause a new subscriber to be added. If we hold the lock, we will deadlock.
            #[allow(clippy::mutable_key_type)]
            let mut subscribers = std::mem::take(&mut *inner.subscribers.lock().unwrap());
            subscribers.retain(|reactive_context| reactive_context.mark_dirty());
            // Extend the subscribers list instead of overwriting it in case a subscriber is added while reactive contexts are marked dirty
            inner.subscribers.lock().unwrap().extend(subscribers);
        }
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
    }

    /// **This pattern is no longer recommended. Prefer [`peek`](Signal::peek) or creating new signals instead.**
    ///
    /// This function is the equivalent of the [write_silent](https://docs.rs/dioxus/latest/dioxus/prelude/struct.UseRef.html#method.write_silent) method on use_ref.
    ///
    /// ## What you should use instead
    ///
    /// ### Reading and Writing to data in the same scope
    ///
    /// Reading and writing to the same signal in the same scope will cause that scope to rerun forever:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// let mut signal = use_signal(|| 0);
    /// // This makes the scope rerun whenever we write to the signal
    /// println!("{}", *signal.read());
    /// // This will rerun the scope because we read the signal earlier in the same scope
    /// *signal.write() += 1;
    /// ```
    ///
    /// You may have used the write_silent method to avoid this infinite loop with use_ref like this:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// let signal = use_signal(|| 0);
    /// // This makes the scope rerun whenever we write to the signal
    /// println!("{}", *signal.read());
    /// // Write silent will not rerun any subscribers
    /// *signal.write_silent() += 1;
    /// ```
    ///
    /// Instead you can use the [`peek`](Signal::peek) and [`write`](Signal::write) methods instead. The peek method will not subscribe to the current scope which will avoid an infinite loop if you are reading and writing to the same signal in the same scope.
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// let mut signal = use_signal(|| 0);
    /// // Peek will read the value but not subscribe to the current scope
    /// println!("{}", *signal.peek());
    /// // Write will update any subscribers which does not include the current scope
    /// *signal.write() += 1;
    /// ```
    ///
    /// ### Reading and Writing to different data
    ///
    ///
    ///
    /// ## Why is this pattern no longer recommended?
    ///
    /// This pattern is no longer recommended because it is very easy to allow your state and UI to grow out of sync. `write_silent` globally opts out of automatic state updates which can be difficult to reason about.
    ///
    ///
    /// Lets take a look at an example:
    /// main.rs:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # fn Child() -> Element { unimplemented!() }
    /// fn app() -> Element {
    ///     let signal = use_context_provider(|| Signal::new(0));
    ///
    ///     // We want to log the value of the signal whenever the app component reruns
    ///     println!("{}", *signal.read());
    ///
    ///     rsx! {
    ///         button {
    ///             // If we don't want to rerun the app component when the button is clicked, we can use write_silent
    ///             onclick: move |_| *signal.write_silent() += 1,
    ///             "Increment"
    ///         }
    ///         Child {}
    ///     }
    /// }
    /// ```
    /// child.rs:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// fn Child() -> Element {
    ///     let signal: Signal<i32> = use_context();
    ///
    ///     // It is difficult to tell that changing the button to use write_silent in the main.rs file will cause UI to be out of sync in a completely different file
    ///     rsx! {
    ///         "{signal}"
    ///     }
    /// }
    /// ```
    ///
    /// Instead [`peek`](Signal::peek) locally opts out of automatic state updates explicitly for a specific read which is easier to reason about.
    ///
    /// Here is the same example using peek:
    /// main.rs:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # fn Child() -> Element { unimplemented!() }
    /// fn app() -> Element {
    ///     let mut signal = use_context_provider(|| Signal::new(0));
    ///
    ///     // We want to log the value of the signal whenever the app component reruns, but we don't want to rerun the app component when the signal is updated so we use peek instead of read
    ///     println!("{}", *signal.peek());
    ///
    ///     rsx! {
    ///         button {
    ///             // We can use write like normal and update the child component automatically
    ///             onclick: move |_| *signal.write() += 1,
    ///             "Increment"
    ///         }
    ///         Child {}
    ///     }
    /// }
    /// ```
    /// child.rs:
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// fn Child() -> Element {
    ///     let signal: Signal<i32> = use_context();
    ///
    ///     rsx! {
    ///         "{signal}"
    ///     }
    /// }
    /// ```
    #[track_caller]
    #[deprecated = "This pattern is no longer recommended. Prefer `peek` or creating new signals instead."]
    pub fn write_silent(&self) -> S::Mut<'static, T> {
        S::map_mut(self.inner.write_unchecked(), |inner| &mut inner.value)
    }
}

impl<T, S: Storage<SignalData<T>>> Readable for Signal<T, S> {
    type Target = T;
    type Storage = S;

    #[track_caller]
    fn try_read_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        let inner = self.inner.try_read_unchecked()?;

        if let Some(reactive_context) = ReactiveContext::current() {
            tracing::trace!("Subscribing to the reactive context {}", reactive_context);
            reactive_context.subscribe(inner.subscribers.clone());
        }

        Ok(S::map(inner, |v| &v.value))
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>> {
        self.inner
            .try_read_unchecked()
            .map(|inner| S::map(inner, |v| &v.value))
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Writable for Signal<T, S> {
    type Mut<'a, R: ?Sized + 'static> = Write<'a, R, S>;

    fn map_mut<I: ?Sized, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Self::Mut<'_, U> {
        Write::map(ref_, f)
    }

    fn try_map_mut<
        I: ?Sized + 'static,
        U: ?Sized + 'static,
        F: FnOnce(&mut I) -> Option<&mut U>,
    >(
        ref_: Self::Mut<'_, I>,
        f: F,
    ) -> Option<Self::Mut<'_, U>> {
        Write::filter_map(ref_, f)
    }

    fn downcast_lifetime_mut<'a: 'b, 'b, R: ?Sized + 'static>(
        mut_: Self::Mut<'a, R>,
    ) -> Self::Mut<'b, R> {
        Write::downcast_lifetime(mut_)
    }

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        #[cfg(debug_assertions)]
        let origin = std::panic::Location::caller();
        self.inner.try_write_unchecked().map(|inner| {
            let borrow = S::map_mut(inner, |v| &mut v.value);
            Write {
                write: borrow,
                drop_signal: Box::new(SignalSubscriberDrop {
                    signal: *self,
                    #[cfg(debug_assertions)]
                    origin,
                }),
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

impl<T> IntoDynNode for Signal<T>
where
    T: Clone + IntoDynNode,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self().into_dyn_node()
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> PartialEq for Signal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Eq for Signal<T, S> {}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone, S: Storage<SignalData<T>> + 'static> Deref for Signal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { Readable::deref_impl(self) }
    }
}

#[cfg(feature = "serialize")]
impl<T: serde::Serialize + 'static, Store: Storage<SignalData<T>>> serde::Serialize
    for Signal<T, Store>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de, T: serde::Deserialize<'de> + 'static, Store: Storage<SignalData<T>>>
    serde::Deserialize<'de> for Signal<T, Store>
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new_maybe_sync(T::deserialize(deserializer)?))
    }
}

/// A mutable reference to a signal's value. This reference acts similarly to [`std::cell::RefMut`], but it has extra debug information
/// and integrates with the reactive system to automatically update dependents.
///
/// [`Write`] implements [`DerefMut`] which means you can call methods on the inner value just like you would on a mutable reference
/// to the inner value. If you need to get the inner reference directly, you can call [`Write::deref_mut`].
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// fn app() -> Element {
///     let mut value = use_signal(|| String::from("hello"));
///     
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 // You call methods like `push_str` on the reference just like you would with the inner String
///                 mutable_reference.push_str("world");
///             },
///             "Click to add world to the string"
///         }
///         div { "{value}" }
///     }
/// }
/// ```
///
/// ## Matching on Write
///
/// You need to get the inner mutable reference with [`Write::deref_mut`] before you match the inner value. If you try to match
/// without calling [`Write::deref_mut`], you will get an error like this:
///
/// ```compile_fail
/// # use dioxus::prelude::*;
/// #[derive(Debug)]
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// fn app() -> Element {
///     let mut value = use_signal(|| Colors::Red(0));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 match mutable_reference {
///                     // Since we are matching on the `Write` type instead of &mut Colors, we can't match on the enum directly
///                     Colors::Red(brightness) => *brightness += 1,
///                     Colors::Green => {}
///                 }
///             },
///             "Click to add brightness to the red color"
///         }
///         div { "{value:?}" }
///     }
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> src/main.rs:18:21
///    |
/// 16 |                 match mutable_reference {
///    |                       ----------------- this expression has type `dioxus::prelude::Write<'_, Colors>`
/// 17 |                     // Since we are matching on the `Write` t...
/// 18 |                     Colors::Red(brightness) => *brightness += 1,
///    |                     ^^^^^^^^^^^^^^^^^^^^^^^ expected `Write<'_, Colors>`, found `Colors`
///    |
///    = note: expected struct `dioxus::prelude::Write<'_, Colors, >`
///                found enum `Colors`
/// ```
///
/// Instead, you need to call deref mut on the reference to get the inner value **before** you match on it:
///
/// ```rust
/// use std::ops::DerefMut;
/// # use dioxus::prelude::*;
/// #[derive(Debug)]
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// fn app() -> Element {
///     let mut value = use_signal(|| Colors::Red(0));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 // DerefMut converts the `Write` into a `&mut Colors`
///                 match mutable_reference.deref_mut() {
///                     // Now we can match on the inner value
///                     Colors::Red(brightness) => *brightness += 1,
///                     Colors::Green => {}
///                 }
///             },
///             "Click to add brightness to the red color"
///         }
///         div { "{value:?}" }
///     }
/// }
/// ```
///
/// ## Generics
/// - T is the current type of the write
/// - S is the storage type of the signal. This type determines if the signal is local to the current thread, or it can be shared across threads.
pub struct Write<'a, T: ?Sized + 'static, S: AnyStorage = UnsyncStorage> {
    write: S::Mut<'a, T>,
    drop_signal: Box<dyn Any>,
}

impl<'a, T: ?Sized + 'static, S: AnyStorage> Write<'a, T, S> {
    /// Map the mutable reference to the signal's value to a new type.
    pub fn map<O: ?Sized>(myself: Self, f: impl FnOnce(&mut T) -> &mut O) -> Write<'a, O, S> {
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
    ) -> Option<Write<'a, O, S>> {
        let Self {
            write, drop_signal, ..
        } = myself;
        let write = S::try_map_mut(write, f);
        write.map(|write| Write { write, drop_signal })
    }

    /// Downcast the lifetime of the mutable reference to the signal's value.
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Mut.  Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    pub fn downcast_lifetime<'b>(mut_: Self) -> Write<'b, T, S>
    where
        'a: 'b,
    {
        Write {
            write: S::downcast_lifetime_mut(mut_.write),
            drop_signal: mut_.drop_signal,
        }
    }
}

impl<T: ?Sized + 'static, S: AnyStorage> Deref for Write<'_, T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T: ?Sized, S: AnyStorage> DerefMut for Write<'_, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

struct SignalSubscriberDrop<T: 'static, S: Storage<SignalData<T>>> {
    signal: Signal<T, S>,
    #[cfg(debug_assertions)]
    origin: &'static std::panic::Location<'static>,
}

#[allow(clippy::no_effect)]
impl<T: 'static, S: Storage<SignalData<T>>> Drop for SignalSubscriberDrop<T, S> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            tracing::trace!(
                "Write on signal at {} finished, updating subscribers",
                self.origin
            );
            crate::warnings::signal_write_in_component_body(self.origin);
            crate::warnings::signal_read_and_write_in_reactive_scope::<T, S>(
                self.origin,
                self.signal,
            );
        }
        self.signal.update_subscribers();
    }
}

fmt_impls!(Signal<T, S: Storage<SignalData<T>>>);
default_impl!(Signal<T, S: Storage<SignalData<T>>>);
write_impls!(Signal<T, S: Storage<SignalData<T>>>);

impl<T: 'static, S: Storage<SignalData<T>>> Clone for Signal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Copy for Signal<T, S> {}
