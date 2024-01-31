use crate::write::Writable;
use std::hash::Hash;

use crate::read::Readable;
use dioxus_core::prelude::*;
use generational_box::{Storage, UnsyncStorage};

use crate::{CopyValue, Effect, ReadOnlySignal, Signal, SignalData};
use rustc_hash::FxHashMap;

/// An object that can efficiently compare a value to a set of values.
#[derive(Debug)]
pub struct Comparer<R: 'static, S: Storage<SignalData<bool>> = UnsyncStorage> {
    subscribers: CopyValue<FxHashMap<R, Signal<bool, S>>>,
}

impl<R: Eq + Hash> Comparer<R> {
    /// Creates a new Comparer which efficiently tracks when a value changes to check if it is equal to a set of values.
    ///
    /// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
    pub fn new(mut f: impl FnMut() -> R + 'static) -> Comparer<R> {
        let subscribers: CopyValue<FxHashMap<R, Signal<bool>>> =
            CopyValue::new(FxHashMap::default());
        let mut previous = CopyValue::new(None);

        Effect::new(move || {
            let subscribers = subscribers.read();
            let mut previous = previous.write();

            if let Some(previous) = previous.take() {
                if let Some(mut value) = subscribers.get(&previous).cloned() {
                    value.set(false)
                }
            }

            let current = f();

            if let Some(mut value) = subscribers.get(&current).cloned() {
                *value.write() = true;
            }

            *previous = Some(current);
        });

        Comparer { subscribers }
    }
}

impl<R: Eq + Hash, S: Storage<SignalData<bool>>> Comparer<R, S> {
    /// Creates a new Comparer that may be `Sync + Send` which efficiently tracks when a value changes to check if it is equal to a set of values.
    ///
    /// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
    pub fn new_maybe_sync(mut f: impl FnMut() -> R + 'static) -> Comparer<R> {
        let subscribers: CopyValue<FxHashMap<R, Signal<bool>>> =
            CopyValue::new(FxHashMap::default());
        let mut previous = CopyValue::new(None);

        Effect::new(move || {
            let subscribers = subscribers.read();
            let mut previous = previous.write();

            if let Some(previous) = previous.take() {
                if let Some(mut value) = subscribers.get(&previous).cloned() {
                    *value.write() = false;
                }
            }

            let current = f();

            if let Some(mut value) = subscribers.get(&current).cloned() {
                *value.write() = true;
            }

            *previous = Some(current);
        });

        Comparer { subscribers }
    }

    /// Returns a signal which is true when the value is equal to the value passed to this function.
    pub fn equal(&mut self, value: R) -> ReadOnlySignal<bool, S> {
        let subscribers = self.subscribers.write();

        match subscribers.get(&value) {
            Some(&signal) => signal.into(),
            None => {
                drop(subscribers);
                let mut subscribers = self.subscribers.write();
                let signal = Signal::new_maybe_sync(false);
                subscribers.insert(value, signal);
                signal.into()
            }
        }
    }
}

impl<R, S: Storage<SignalData<bool>>> Clone for Comparer<R, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R, S: Storage<SignalData<bool>>> Copy for Comparer<R, S> {}

/// Creates a new Comparer which efficiently tracks when a value changes to check if it is equal to a set of values.
///
/// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut count = use_signal(cx, || 0);
///     let comparer = use_comparer(cx, move || count.value());
///
///     render! {
///         for i in 0..10 {
///             // Child will only re-render when i == count
///             Child { active: comparer.equal(i) }
///         }
///         button {
///             // This will only rerender the child with the old and new value of i == count
///             // Because we are using a comparer, this will be O(1) instead of the O(n) performance of a selector
///             onclick: move |_| count += 1,
///             "Increment"
///         }
///     }
/// }
///
/// #[component]
/// fn Child(cx: Scope, active: ReadOnlySignal<bool>) -> Element {
///     if *active() {
///         render! { "Active" }
///     } else {
///         render! { "Inactive" }
///     }
/// }
/// ```
#[must_use]
pub fn use_comparer<R: Eq + Hash>(f: impl FnMut() -> R + 'static) -> Comparer<R> {
    use_hook(move || Comparer::new(f))
}
