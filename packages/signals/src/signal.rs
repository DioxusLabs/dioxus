use crate::MappedSignal;
use std::{
    cell::RefCell,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use dioxus_core::{
    prelude::{
        current_scope_id, has_context, provide_context, schedule_update_any, use_hook,
        IntoAttributeValue,
    },
    ScopeId,
};
use generational_box::{
    GenerationalBoxId, Mappable, MappableMut, Storage, SyncStorage, UnsyncStorage,
};
use parking_lot::RwLock;

use crate::{get_effect_ref, CopyValue, EffectStackRef, EFFECT_STACK};

/// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
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
#[track_caller]
#[must_use]
pub fn use_signal<T: 'static>(f: impl FnOnce() -> T) -> Signal<T, UnsyncStorage> {
    #[cfg(debug_assertions)]
    let caller = std::panic::Location::caller();

    use_hook(|| {
        Signal::new_with_caller(
            f(),
            #[cfg(debug_assertions)]
            caller,
        )
    })
}

/// Creates a new `Send + Sync`` Signal. Signals are a Copy state management solution with automatic dependency tracking.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut count = use_signal_sync(cx, || 0);
///
///     // Because signals have automatic dependency tracking, if you never read them in a component, that component will not be re-rended when the signal is updated.
///     // The app component will never be rerendered in this example.
///     render! { Child { state: count } }
/// }
///
/// #[component]
/// fn Child(cx: Scope, state: Signal<u32, SyncStorage>) -> Element {
///     let state = *state;
///
///     use_future!(cx,  |()| async move {
///         // This signal is Send + Sync, so we can use it in an another thread
///         tokio::spawn(async move {
///             // Because the signal is a Copy type, we can use it in an async block without cloning it.
///             *state.write() += 1;
///         }).await;
///     });
///
///     render! {
///         button {
///             onclick: move |_| *state.write() += 1,
///             "{state}"
///         }
///     }
/// }
/// ```
#[must_use]
#[track_caller]
pub fn use_signal_sync<T: Send + Sync + 'static>(f: impl FnOnce() -> T) -> Signal<T, SyncStorage> {
    #[cfg(debug_assertions)]
    let caller = std::panic::Location::caller();
    use_hook(|| {
        Signal::new_with_caller(
            f(),
            #[cfg(debug_assertions)]
            caller,
        )
    })
}

struct Unsubscriber {
    scope: ScopeId,
    subscribers: UnsubscriberArray,
}

type UnsubscriberArray = Vec<Rc<RefCell<Vec<ScopeId>>>>;

impl Drop for Unsubscriber {
    fn drop(&mut self) {
        for subscribers in &self.subscribers {
            subscribers.borrow_mut().retain(|s| *s != self.scope);
        }
    }
}

fn current_unsubscriber() -> Rc<RefCell<Unsubscriber>> {
    match has_context() {
        Some(rt) => rt,
        None => {
            let owner = Unsubscriber {
                scope: current_scope_id().expect("in a virtual dom"),
                subscribers: Default::default(),
            };
            provide_context(Rc::new(RefCell::new(owner)))
        }
    }
}

#[derive(Default)]
pub(crate) struct SignalSubscribers {
    pub(crate) subscribers: Vec<ScopeId>,
    pub(crate) effect_subscribers: Vec<GenerationalBoxId>,
}

/// The data stored for tracking in a signal.
pub struct SignalData<T> {
    pub(crate) subscribers: Arc<RwLock<SignalSubscribers>>,
    pub(crate) update_any: Arc<dyn Fn(ScopeId) + Sync + Send>,
    pub(crate) effect_ref: EffectStackRef,
    pub(crate) value: T,
}

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

#[cfg(feature = "serde")]
impl<T: serde::Serialize + 'static> serde::Serialize for Signal<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.read().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de> + 'static> serde::Deserialize<'de> for Signal<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(T::deserialize(deserializer)?))
    }
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
}

impl<T: 'static, S: Storage<SignalData<T>>> Signal<T, S> {
    /// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
    #[track_caller]
    #[tracing::instrument(skip(value))]
    pub fn new_maybe_sync(value: T) -> Self {
        Self {
            inner: CopyValue::<SignalData<T>, S>::new_maybe_sync(SignalData {
                subscribers: Default::default(),
                update_any: schedule_update_any(),
                value,
                effect_ref: get_effect_ref(),
            }),
        }
    }

    /// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
    fn new_with_caller(
        value: T,
        #[cfg(debug_assertions)] caller: &'static std::panic::Location<'static>,
    ) -> Self {
        Self {
            inner: CopyValue::new_with_caller(
                SignalData {
                    subscribers: Default::default(),
                    update_any: schedule_update_any(),
                    value,
                    effect_ref: get_effect_ref(),
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
                    update_any: schedule_update_any(),
                    value,
                    effect_ref: get_effect_ref(),
                },
                owner,
            ),
        }
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.  If you would like to read the signal without subscribing to it, you can use [`Self::peek`] instead.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn read(
        &self,
    ) -> <<S as Storage<SignalData<T>>>::Ref as Mappable<SignalData<T>>>::Mapped<T> {
        let inner = self.inner.read();
        if let Some(effect) = EFFECT_STACK.with(|stack| stack.current()) {
            let subscribers = inner.subscribers.read();
            if !subscribers.effect_subscribers.contains(&effect.inner.id()) {
                drop(subscribers);
                let mut subscribers = inner.subscribers.write();
                subscribers.effect_subscribers.push(effect.inner.id());
            }
        } else if let Some(current_scope_id) = current_scope_id() {
            // only subscribe if the vdom is rendering
            if dioxus_core::vdom_is_rendering() {
                tracing::trace!(
                    "{:?} subscribed to {:?}",
                    self.inner.value,
                    current_scope_id
                );
                let subscribers = inner.subscribers.read();
                if !subscribers.subscribers.contains(&current_scope_id) {
                    drop(subscribers);
                    let mut subscribers = inner.subscribers.write();
                    subscribers.subscribers.push(current_scope_id);
                    let unsubscriber = current_unsubscriber();
                    subscribers.subscribers.push(unsubscriber.borrow().scope);
                }
            }
        }
        S::Ref::map(inner, |v| &v.value)
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    pub fn peek(
        &self,
    ) -> <<S as Storage<SignalData<T>>>::Ref as Mappable<SignalData<T>>>::Mapped<T> {
        let inner = self.inner.read();
        S::Ref::map(inner, |v| &v.value)
    }

    /// Get a mutable reference to the signal's value.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn write<'a>(
        &'a mut self,
    ) -> Write<T, <<S as Storage<SignalData<T>>>::Mut as MappableMut<SignalData<T>>>::Mapped<T>, S>
    {
        self.write_unchecked()
    }

    /// Write to the value through an immutable reference.
    ///
    /// This is public since it's useful in many scenarios, but we generally recommend mutation through [`Self::write`] instead.
    #[track_caller]
    pub fn write_unchecked(
        &self,
    ) -> Write<T, <<S as Storage<SignalData<T>>>::Mut as MappableMut<SignalData<T>>>::Mapped<T>, S>
    {
        let inner = self.inner.write();
        let borrow = S::Mut::map(inner, |v| &mut v.value);
        Write {
            write: borrow,
            signal: SignalSubscriberDrop { signal: *self },
            phantom: std::marker::PhantomData,
        }
    }

    fn update_subscribers(&self) {
        {
            let inner = self.inner.read();
            for &scope_id in &*inner.subscribers.read().subscribers {
                tracing::trace!(
                    "Write on {:?} triggered update on {:?}",
                    self.inner.value,
                    scope_id
                );
                (inner.update_any)(scope_id);
            }
        }

        let self_read = &self.inner.read();
        let subscribers = {
            let effects = &mut self_read.subscribers.write().effect_subscribers;
            std::mem::take(&mut *effects)
        };
        let effect_ref = &self_read.effect_ref;
        for effect in subscribers {
            tracing::trace!(
                "Write on {:?} triggered effect {:?}",
                self.inner.value,
                effect
            );
            effect_ref.rerun_effect(effect);
        }
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    pub fn set(&mut self, value: T) {
        *self.write() = value;
    }

    /// Set the value of the signal without triggering an update on subscribers.
    ///
    /// todo: we should make it so setting while rendering doesn't trigger an update s
    pub fn set_untracked(&self, value: T) {
        let mut inner = self.inner.write();
        inner.value = value;
    }

    /// Run a closure with a reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&mut self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }

    /// Map the signal to a new type.
    pub fn map<O>(
        self,
        f: impl Fn(&T) -> &O + 'static,
    ) -> MappedSignal<
        <<<S as generational_box::Storage<SignalData<T>>>::Ref as generational_box::Mappable<
            SignalData<T>,
        >>::Mapped<T> as generational_box::Mappable<T>>::Mapped<O>,
    > {
        MappedSignal::new(self, f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
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

impl<T: Clone + 'static, S: Storage<SignalData<T>>> Signal<T, S> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn cloned(&self) -> T {
        self.read().clone()
    }
}

impl<S: Storage<SignalData<bool>>> Signal<bool, S> {
    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    pub fn toggle(&mut self) {
        self.set(!self.cloned());
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
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() }).clone();

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe { std::mem::transmute(self) },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &Self::Target
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

/// A mutable reference to a signal's value.
///
/// T is the current type of the write
/// B is the dynamically checked type of the write (RefMut)
/// S is the storage type of the signal
/// I is the type of the original signal
pub struct Write<T: 'static, B: MappableMut<T>, S: Storage<SignalData<I>>, I: 'static = T> {
    write: B,
    signal: SignalSubscriberDrop<I, S>,
    phantom: std::marker::PhantomData<T>,
}

impl<T: 'static, B: MappableMut<T>, S: Storage<SignalData<I>>, I: 'static> Write<T, B, S, I> {
    /// Map the mutable reference to the signal's value to a new type.
    pub fn map<O>(myself: Self, f: impl FnOnce(&mut T) -> &mut O) -> Write<O, B::Mapped<O>, S, I> {
        let Self { write, signal, .. } = myself;
        Write {
            write: B::map(write, f),
            signal,
            phantom: std::marker::PhantomData,
        }
    }

    /// Try to map the mutable reference to the signal's value to a new type
    pub fn filter_map<O>(
        myself: Self,
        f: impl FnOnce(&mut T) -> Option<&mut O>,
    ) -> Option<Write<O, B::Mapped<O>, S, I>> {
        let Self { write, signal, .. } = myself;
        let write = B::try_map(write, f);
        write.map(|write| Write {
            write,
            signal,
            phantom: PhantomData,
        })
    }
}

impl<T: 'static, B: MappableMut<T>, S: Storage<SignalData<I>>, I: 'static> Deref
    for Write<T, B, S, I>
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T, B: MappableMut<T>, S: Storage<SignalData<I>>, I> DerefMut for Write<T, B, S, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

/// A signal that can only be read from.
pub struct ReadOnlySignal<T: 'static, S: Storage<SignalData<T>> = UnsyncStorage> {
    inner: Signal<T, S>,
}

impl<T: 'static, S: Storage<SignalData<T>>> From<Signal<T, S>> for ReadOnlySignal<T, S> {
    fn from(inner: Signal<T, S>) -> Self {
        Self { inner }
    }
}

impl<T: 'static> ReadOnlySignal<T> {
    /// Create a new read-only signal.
    #[track_caller]
    pub fn new(signal: Signal<T>) -> Self {
        Self::new_maybe_sync(signal)
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> ReadOnlySignal<T, S> {
    /// Create a new read-only signal that is maybe sync.
    #[track_caller]
    pub fn new_maybe_sync(signal: Signal<T, S>) -> Self {
        Self { inner: signal }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn read(
        &self,
    ) -> <<S as Storage<SignalData<T>>>::Ref as Mappable<SignalData<T>>>::Mapped<T> {
        self.inner.read()
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    pub fn peek(
        &self,
    ) -> <<S as Storage<SignalData<T>>>::Ref as Mappable<SignalData<T>>>::Mapped<T> {
        self.inner.peek()
    }

    /// Run a closure with a reference to the signal's value.
    #[track_caller]
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.inner.with(f)
    }
}

impl<T: Clone + 'static, S: Storage<SignalData<T>>> ReadOnlySignal<T, S> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> PartialEq for ReadOnlySignal<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T: Copy, S: Storage<SignalData<T>> + 'static> Deref for ReadOnlySignal<T, S> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || *Self::read(unsafe { &*uninit_callable.as_ptr() });

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe { std::mem::transmute(self) },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &Self::Target
    }
}
