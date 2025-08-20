use crate::{write::WritableExt, ReadableExt};
use std::hash::Hash;

use dioxus_core::ReactiveContext;
use futures_util::StreamExt;
use generational_box::{Storage, UnsyncStorage};

use crate::{CopyValue, ReadSignal, Signal, SignalData};
use rustc_hash::FxHashMap;

/// An object that can efficiently compare a value to a set of values.
pub struct SetCompare<R, S: 'static = UnsyncStorage> {
    subscribers: CopyValue<FxHashMap<R, Signal<bool, S>>>,
}

impl<R: Eq + Hash + 'static> SetCompare<R> {
    /// Creates a new [`SetCompare`] which efficiently tracks when a value changes to check if it is equal to a set of values.
    ///
    /// Generally, you shouldn't need to use this hook. Instead you can use [`crate::Memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
    #[track_caller]
    pub fn new(f: impl FnMut() -> R + 'static) -> SetCompare<R> {
        Self::new_maybe_sync(f)
    }
}

impl<R: Eq + Hash + 'static, S: Storage<SignalData<bool>> + 'static> SetCompare<R, S> {
    /// Creates a new [`SetCompare`] that may be `Sync + Send` which efficiently tracks when a value changes to check if it is equal to a set of values.
    ///
    /// Generally, you shouldn't need to use this hook. Instead you can use [`crate::Memo`]. If you have many values that you need to compare to a single value, this hook will change updates from O(n) to O(1) where n is the number of values you are comparing to.
    #[track_caller]
    pub fn new_maybe_sync(mut f: impl FnMut() -> R + 'static) -> SetCompare<R, S> {
        let subscribers: CopyValue<FxHashMap<R, Signal<bool, S>>> =
            CopyValue::new(FxHashMap::default());
        let mut previous = CopyValue::new(None);

        let mut recompute = move || {
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
        };
        let (rc, mut changed) = ReactiveContext::new();
        dioxus_core::spawn(async move {
            loop {
                // Recompute the value
                rc.reset_and_run_in(&mut recompute);

                // Wait for context to change
                let _ = changed.next().await;
            }
        });

        SetCompare { subscribers }
    }
}

impl<R: Eq + Hash + 'static> SetCompare<R> {
    /// Returns a signal which is true when the value is equal to the value passed to this function.
    pub fn equal(&mut self, value: R) -> ReadSignal<bool> {
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

impl<R, S: Storage<SignalData<bool>>> PartialEq for SetCompare<R, S> {
    fn eq(&self, other: &Self) -> bool {
        self.subscribers == other.subscribers
    }
}

impl<R, S: Storage<SignalData<bool>>> Clone for SetCompare<R, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R, S: Storage<SignalData<bool>>> Copy for SetCompare<R, S> {}
