use std::hash::Hash;

use dioxus_core::prelude::*;

use crate::{CopyValue, Effect, ReadOnlySignal, Signal};
use rustc_hash::FxHashMap;

/// An object that can efficiently compare a value to a set of values.
#[derive(Debug)]
pub struct Comparer<R: 'static> {
    subscribers: CopyValue<FxHashMap<R, Signal<bool>>>,
}

impl<R: Eq + Hash> Comparer<R> {
    /// Returns a signal which is true when the value is equal to the value passed to this function.
    pub fn equal(&self, value: R) -> ReadOnlySignal<bool> {
        let subscribers = self.subscribers.read();

        match subscribers.get(&value) {
            Some(&signal) => signal.into(),
            None => {
                drop(subscribers);
                let mut subscribers = self.subscribers.write();
                let signal = Signal::new(false);
                subscribers.insert(value, signal.clone());
                signal.into()
            }
        }
    }
}

impl<R> Clone for Comparer<R> {
    fn clone(&self) -> Self {
        Self {
            subscribers: self.subscribers.clone(),
        }
    }
}

impl<R> Copy for Comparer<R> {}

/// Creates a new Comparer which efficiently tracks when a value changes to check if it is equal to a set of values.
///
/// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_selector`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
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
pub fn use_comparer<R: Eq + Hash>(cx: &ScopeState, f: impl FnMut() -> R + 'static) -> Comparer<R> {
    *cx.use_hook(move || comparer(f))
}

/// Creates a new Comparer which efficiently tracks when a value changes to check if it is equal to a set of values.
///
/// Generally, you shouldn't need to use this hook. Instead you can use [`crate::use_selector`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
pub fn comparer<R: Eq + Hash>(mut f: impl FnMut() -> R + 'static) -> Comparer<R> {
    let subscribers: CopyValue<FxHashMap<R, Signal<bool>>> = CopyValue::new(FxHashMap::default());
    let previous = CopyValue::new(None);

    Effect::new(move || {
        let subscribers = subscribers.read();
        let mut previous = previous.write();

        if let Some(previous) = previous.take() {
            if let Some(value) = subscribers.get(&previous) {
                value.set(false);
            }
        }

        let current = f();

        if let Some(value) = subscribers.get(&current) {
            value.set(true);
        }

        *previous = Some(current);
    });

    Comparer { subscribers }
}
