use crate::{
    read::Readable, write::Writable, Effect, EffectInner, GlobalMemo, GlobalSignal, MappedSignal,
    ReadOnlySignal,
};
use std::{
    any::Any,
    cell::RefCell,
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
use generational_box::{AnyStorage, GenerationalBoxId, Storage, SyncStorage, UnsyncStorage};
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

/// A signal that can safely shared between threads.
pub type SyncSignal<T> = Signal<T, SyncStorage>;

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
    pub fn selector(f: impl FnMut() -> T + 'static) -> ReadOnlySignal<T> {
        Self::maybe_sync_memo(f)
    }

    /// Creates a new Selector that may be Sync + Send. The selector will be run immediately and whenever any signal it reads changes.
    ///
    /// Selectors can be used to efficiently compute derived data from signals.
    #[track_caller]
    pub fn maybe_sync_memo<S: Storage<SignalData<T>>>(
        mut f: impl FnMut() -> T + 'static,
    ) -> ReadOnlySignal<T, S> {
        let effect = Effect {
            source: current_scope_id().expect("in a virtual dom"),
            inner: CopyValue::invalid(),
        };

        {
            EFFECT_STACK.with(|stack| stack.effects.write().push(effect));
        }
        let mut state: Signal<T, S> = Signal::new_maybe_sync(f());
        {
            EFFECT_STACK.with(|stack| stack.effects.write().pop());
        }

        let invalid_id = effect.id();
        tracing::trace!("Creating effect: {:?}", invalid_id);
        effect.inner.value.set(EffectInner {
            callback: Box::new(move || {
                let value = f();
                let changed = {
                    let old = state.inner.read();
                    value != old.value
                };
                if changed {
                    state.set(value)
                }
            }),
            id: invalid_id,
        });
        {
            EFFECT_STACK.with(|stack| stack.effect_mapping.write().insert(invalid_id, effect));
        }

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

    /// Map the signal to a new type.
    pub fn map<O>(self, f: impl Fn(&T) -> &O + 'static) -> MappedSignal<S::Ref<O>> {
        MappedSignal::new(self, f)
    }

    /// Get the generational id of the signal.
    pub fn id(&self) -> generational_box::GenerationalBoxId {
        self.inner.id()
    }
}

impl<T, S: Storage<SignalData<T>>> Readable<T> for Signal<T, S> {
    type Ref<R: ?Sized + 'static> = S::Ref<R>;

    fn map_ref<I, U: ?Sized, F: FnOnce(&I) -> &U>(ref_: Self::Ref<I>, f: F) -> Self::Ref<U> {
        S::map(ref_, f)
    }

    fn try_map_ref<I, U: ?Sized, F: FnOnce(&I) -> Option<&U>>(
        ref_: Self::Ref<I>,
        f: F,
    ) -> Option<Self::Ref<U>> {
        S::try_map(ref_, f)
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.  If you would like to read the signal without subscribing to it, you can use [`Self::peek`] instead.
    ///
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    fn read(&self) -> S::Ref<T> {
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
        S::map(inner, |v| &v.value)
    }

    /// Get the current value of the signal. **Unlike read, this will not subscribe the current scope to the signal which can cause parts of your UI to not update.**
    ///
    /// If the signal has been dropped, this will panic.
    fn peek(&self) -> S::Ref<T> {
        let inner = self.inner.read();
        S::map(inner, |v| &v.value)
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Writable<T> for Signal<T, S> {
    type Mut<R: ?Sized + 'static> = Write<R, S>;

    fn map_mut<I, U: ?Sized + 'static, F: FnOnce(&mut I) -> &mut U>(
        ref_: Self::Mut<I>,
        f: F,
    ) -> Self::Mut<U> {
        Write::map(ref_, f)
    }

    fn try_map_mut<I: 'static, U: ?Sized + 'static, F: FnOnce(&mut I) -> Option<&mut U>>(
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
