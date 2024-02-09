use dioxus_core::prelude::{provide_root_context, try_consume_context};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

mod memo;
pub use memo::*;

mod signal;
pub use signal::*;

#[derive(Clone)]
pub(crate) struct GlobalSignalContext {
    signal: Rc<RefCell<HashMap<*const (), Box<dyn Any>>>>,
}

pub(crate) fn get_global_context() -> GlobalSignalContext {
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
