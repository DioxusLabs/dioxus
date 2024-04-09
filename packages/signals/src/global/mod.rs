use dioxus_core::prelude::{provide_root_context, try_consume_context};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

mod memo;
pub use memo::*;

mod signal;
pub use signal::*;

use crate::Signal;

#[derive(Clone)]
pub struct GlobalSignalContext {
    signal: Rc<RefCell<HashMap<GlobalSignalContextKey, Box<dyn Any>>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlobalSignalContextKey {
    Ptr(*const ()),
    Key(&'static str),
}

impl GlobalSignalContext {
    pub fn get_signal_with_key<T>(&self, key: &str) -> Signal<T> {
        // temporarily pretend it's a static str
        // todo: maybe don't do this! use a string for a key or something
        let _key = unsafe { std::mem::transmute::<&str, &'static str>(key) };

        self.signal
            .borrow()
            .get(&GlobalSignalContextKey::Key(_key))
            .map(|f| f.downcast_ref::<Signal<T>>().unwrap().clone())
            .unwrap()
    }
}

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
