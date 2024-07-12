use dioxus_core::prelude::{provide_root_context, try_consume_context};
use std::{any::Any, cell::RefCell, collections::HashMap, panic::Location, rc::Rc};

mod memo;
pub use memo::*;

mod signal;
pub use signal::*;

use crate::Signal;

/// The context for global signals
#[derive(Clone)]
pub struct GlobalSignalContext {
    signal: Rc<RefCell<HashMap<GlobalKey, Box<dyn Any>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GlobalKey {
    Static(&'static Location<'static>),
    Dynamic(&'static str),
}

impl GlobalSignalContext {
    /// Get a signal with the given string key
    /// The key will be converted to a UUID with the appropriate internal namespace
    pub fn get_signal_with_key<T>(&self, key: &str) -> Option<Signal<T>> {
        // temporarily extend the lifetime
        let key = unsafe { std::mem::transmute::<&str, &'static str>(key) };

        let id = GlobalKey::Dynamic(key);

        self.signal.borrow().get(&id).map(|f| {
            f.downcast_ref::<Signal<T>>()
                .unwrap_or_else(|| {
                    panic!(
                        "Global signal with key {:?} is not of the expected type. Keys are {:?}",
                        key,
                        self.signal.borrow().keys()
                    )
                })
                .clone()
        })
    }
}

/// Get the global context for signals
pub fn get_global_context() -> GlobalSignalContext {
    match try_consume_context() {
        Some(context) => context,
        None => {
            let context = GlobalSignalContext {
                signal: Rc::new(RefCell::new(HashMap::new())),
            };
            provide_root_context(context)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that keys of global signals are correctly generated and different from one another.
    /// We don't want signals to merge, but we also want them to use both string IDs and memory addresses.
    #[test]
    fn test_global_keys() {
        // we're using consts since it's harder than statics due to merging - these won't be merged
        const MYSIGNAL: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL2: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL3: GlobalSignal<i32> = GlobalSignal::with_key(|| 42, "custom-keyed");

        let a = MYSIGNAL.key();
        let b = MYSIGNAL.key();
        let c = (&&&&MYSIGNAL).key();
        assert_eq!(a, b);
        assert_eq!(b, c);

        let d = MYSIGNAL2.key();
        assert_ne!(a, d);

        let e = MYSIGNAL3.key();
        assert_ne!(a, e);
    }
}
