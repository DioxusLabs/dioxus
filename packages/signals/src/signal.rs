use std::{
    cell::{Ref, RefCell, RefMut},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use dioxus_core::{
    prelude::{current_scope_id, has_context, provide_context, schedule_update_any},
    ScopeId, ScopeState,
};

use crate::{get_effect_stack, CopyValue, Effect, EffectStack};

/// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut count = use_signal(cx, || 0);
///
///     // Because signals have automatic dependency tracking, if you never read them in a component, that component will not be re-rended when the signal is updated.
///     // The app component will never be rerendered in this example.
///     render! { Child { state: count } }
/// }
///
/// #[component]
/// fn Child(cx: Scope, state: Signal<u32>) -> Element {
///     let state = *state;
///
///     use_future!(cx,  |()| async move {
///         // Because the signal is a Copy type, we can use it in an async block without cloning it.
///         *state.write() += 1;
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
pub fn use_signal<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) -> Signal<T> {
    *cx.use_hook(|| Signal::new(f()))
}

#[derive(Clone)]
struct Unsubscriber {
    scope: ScopeId,
    subscribers: UnsubscriberArray,
}

type UnsubscriberArray = Rc<RefCell<Vec<Rc<RefCell<Vec<ScopeId>>>>>>;

impl Drop for Unsubscriber {
    fn drop(&mut self) {
        for subscribers in self.subscribers.borrow().iter() {
            subscribers.borrow_mut().retain(|s| *s != self.scope);
        }
    }
}

fn current_unsubscriber() -> Unsubscriber {
    match has_context() {
        Some(rt) => rt,
        None => {
            let owner = Unsubscriber {
                scope: current_scope_id().expect("in a virtual dom"),
                subscribers: Default::default(),
            };
            provide_context(owner).expect("in a virtual dom")
        }
    }
}

pub(crate) struct SignalData<T> {
    pub(crate) subscribers: Rc<RefCell<Vec<ScopeId>>>,
    pub(crate) effect_subscribers: Rc<RefCell<Vec<Effect>>>,
    pub(crate) update_any: Arc<dyn Fn(ScopeId)>,
    pub(crate) effect_stack: EffectStack,
    pub(crate) value: T,
}

/// Creates a new Signal. Signals are a Copy state management solution with automatic dependency tracking.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// #[component]
/// fn App(cx: Scope) -> Element {
///     let mut count = use_signal(cx, || 0);
///
///     // Because signals have automatic dependency tracking, if you never read them in a component, that component will not be re-rended when the signal is updated.
///     // The app component will never be rerendered in this example.
///     render! { Child { state: count } }
/// }
///
/// #[component]
/// fn Child(cx: Scope, state: Signal<u32>) -> Element {
///     let state = *state;
///
///     use_future!(cx,  |()| async move {
///         // Because the signal is a Copy type, we can use it in an async block without cloning it.
///         *state.write() += 1;
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
pub struct Signal<T: 'static> {
    pub(crate) inner: CopyValue<SignalData<T>>,
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
    pub fn new(value: T) -> Self {
        Self {
            inner: CopyValue::new(SignalData {
                subscribers: Default::default(),
                effect_subscribers: Default::default(),
                update_any: schedule_update_any().expect("in a virtual dom"),
                value,
                effect_stack: get_effect_stack(),
            }),
        }
    }

    /// Create a new signal with a custom owner scope. The signal will be dropped when the owner scope is dropped instead of the current scope.
    pub fn new_in_scope(value: T, owner: ScopeId) -> Self {
        Self {
            inner: CopyValue::new_in_scope(
                SignalData {
                    subscribers: Default::default(),
                    effect_subscribers: Default::default(),
                    update_any: schedule_update_any().expect("in a virtual dom"),
                    value,
                    effect_stack: get_effect_stack(),
                },
                owner,
            ),
        }
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    /// If the signal has been dropped, this will panic.
    pub fn read(&self) -> Ref<T> {
        let inner = self.inner.read();
        if let Some(effect) = inner.effect_stack.current() {
            let mut effect_subscribers = inner.effect_subscribers.borrow_mut();
            if !effect_subscribers.contains(&effect) {
                effect_subscribers.push(effect);
            }
        } else if let Some(current_scope_id) = current_scope_id() {
            // only subscribe if the vdom is rendering
            if dioxus_core::vdom_is_rendering() {
                tracing::trace!(
                    "{:?} subscribed to {:?}",
                    self.inner.value,
                    current_scope_id
                );
                let mut subscribers = inner.subscribers.borrow_mut();
                if !subscribers.contains(&current_scope_id) {
                    subscribers.push(current_scope_id);
                    drop(subscribers);
                    let unsubscriber = current_unsubscriber();
                    inner.subscribers.borrow_mut().push(unsubscriber.scope);
                }
            }
        }
        Ref::map(inner, |v| &v.value)
    }

    /// Get a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    pub fn write(&self) -> Write<'_, T> {
        let inner = self.inner.write();
        let borrow = RefMut::map(inner, |v| &mut v.value);
        Write {
            write: borrow,
            signal: SignalSubscriberDrop { signal: *self },
        }
    }

    fn update_subscribers(&self) {
        {
            let inner = self.inner.read();
            for &scope_id in &*inner.subscribers.borrow() {
                tracing::trace!(
                    "Write on {:?} triggered update on {:?}",
                    self.inner.value,
                    scope_id
                );
                (inner.update_any)(scope_id);
            }
        }

        let subscribers = {
            let self_read = self.inner.read();
            let mut effects = self_read.effect_subscribers.borrow_mut();
            std::mem::take(&mut *effects)
        };
        for effect in subscribers {
            tracing::trace!(
                "Write on {:?} triggered effect {:?}",
                self.inner.value,
                effect
            );
            effect.try_run();
        }
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    pub fn set(&self, value: T) {
        *self.write() = value;
    }

    /// Run a closure with a reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        let write = self.read();
        f(&*write)
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut T) -> O) -> O {
        let mut write = self.write();
        f(&mut *write)
    }
}

impl<T: Clone + 'static> Signal<T> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    /// If the signal has been dropped, this will panic.
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl Signal<bool> {
    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    pub fn toggle(&self) {
        self.set(!self.value());
    }
}

impl<T: 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Deref for Signal<T> {
    type Target = dyn Fn() -> Ref<'static, T>;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() });

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

struct SignalSubscriberDrop<T: 'static> {
    signal: Signal<T>,
}

impl<T: 'static> Drop for SignalSubscriberDrop<T> {
    fn drop(&mut self) {
        self.signal.update_subscribers();
    }
}

/// A mutable reference to a signal's value.
pub struct Write<'a, T: 'static, I: 'static = T> {
    write: RefMut<'a, T>,
    signal: SignalSubscriberDrop<I>,
}

impl<'a, T: 'static, I: 'static> Write<'a, T, I> {
    /// Map the mutable reference to the signal's value to a new type.
    pub fn map<O>(myself: Self, f: impl FnOnce(&mut T) -> &mut O) -> Write<'a, O, I> {
        let Self { write, signal } = myself;
        Write {
            write: RefMut::map(write, f),
            signal,
        }
    }

    /// Try to map the mutable reference to the signal's value to a new type
    pub fn filter_map<O>(
        myself: Self,
        f: impl FnOnce(&mut T) -> Option<&mut O>,
    ) -> Option<Write<'a, O, I>> {
        let Self { write, signal } = myself;
        let write = RefMut::filter_map(write, f).ok();
        write.map(|write| Write { write, signal })
    }
}

impl<'a, T: 'static, I: 'static> Deref for Write<'a, T, I> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T, I> DerefMut for Write<'_, T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

/// A signal that can only be read from.
pub struct ReadOnlySignal<T: 'static> {
    inner: Signal<T>,
}

impl<T: 'static> ReadOnlySignal<T> {
    /// Create a new read-only signal.
    pub fn new(signal: Signal<T>) -> Self {
        Self { inner: signal }
    }

    /// Get the scope that the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        self.inner.origin_scope()
    }

    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn read(&self) -> Ref<T> {
        self.inner.read()
    }

    /// Run a closure with a reference to the signal's value.
    pub fn with<O>(&self, f: impl FnOnce(&T) -> O) -> O {
        self.inner.with(f)
    }
}

impl<T: Clone + 'static> ReadOnlySignal<T> {
    /// Get the current value of the signal. This will subscribe the current scope to the signal.
    pub fn value(&self) -> T {
        self.read().clone()
    }
}

impl<T: 'static> PartialEq for ReadOnlySignal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Deref for ReadOnlySignal<T> {
    type Target = dyn Fn() -> Ref<'static, T>;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::read(unsafe { &*uninit_callable.as_ptr() });

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
