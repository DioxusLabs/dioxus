use std::{any::TypeId, rc::Rc};

use dioxus_core::{
    prelude::{provide_context, provide_root_context},
    use_hook, Element,
};
use dioxus_devtools::Devtools;

static mut __HOTRELOAD_APP: Option<fn() -> Element> = None;

pub fn set_app(app: fn() -> Element) -> fn() -> Element {
    unsafe {
        __HOTRELOAD_APP = Some(app);
    }

    __hotreload_main as fn() -> Element
}

#[no_mangle]
pub fn __hotreload_main() -> Element {
    let devtools = use_hook(|| {
        println!(
            "providing devtools context in scope {:?}: {:?}",
            dioxus_core::prelude::current_scope_id(),
            TypeId::of::<Rc<Devtools>>()
        );
        let app = unsafe { __HOTRELOAD_APP.unwrap() };
        provide_root_context(Rc::new(dioxus_devtools::Devtools::new(app)))
    });

    unsafe { __HOTRELOAD_APP.unwrap()() }
}
