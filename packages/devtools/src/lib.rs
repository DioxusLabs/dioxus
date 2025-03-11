use std::{any::TypeId, cell::Cell, ffi::CString, rc::Rc};

use dioxus_core::{
    prelude::{consume_context, try_consume_context},
    Element, ScopeId, VirtualDom,
};
pub use dioxus_devtools_types::*;
use dioxus_signals::{GlobalKey, Writable};
use libc::dlsym;
use subsecond::JumpTable;
use warnings::Warning;

pub struct Devtools {
    main_fn: Cell<fn() -> Element>,
}

impl Devtools {
    pub fn new(entry: fn() -> Element) -> Self {
        Self {
            main_fn: Cell::new(entry),
        }
    }

    pub fn main_fn(&self) -> fn() -> Element {
        self.main_fn.get()
    }
}

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) {
    dom.runtime().on_scope(ScopeId::ROOT, || {
        // Update signals...
        let ctx = dioxus_signals::get_global_context();

        for template in &msg.templates {
            let value = template.template.clone();
            let key = GlobalKey::File {
                file: template.key.file.as_str(),
                line: template.key.line as _,
                column: template.key.column as _,
                index: template.key.index as _,
            };
            if let Some(mut signal) = ctx.get_signal_with_key(key.clone()) {
                dioxus_signals::warnings::signal_read_and_write_in_reactive_scope::allow(|| {
                    dioxus_signals::warnings::signal_write_in_component_body::allow(|| {
                        signal.set(Some(value));
                    });
                });
            }
        }

        // Patch the binary
        println!(
            "patching binary, looking for id in scope {:?}: {:?}",
            dioxus_core::prelude::current_scope_id(),
            TypeId::of::<Rc<Devtools>>()
        );
        if let Some(devtools) = try_consume_context::<Rc<Devtools>>() {
            println!("using devtools context with patch {:?}", msg.patch);
            if let Some(so) = msg.patch.clone() {
                // let jump_table = msg.jump_table.clone().unwrap();
                // let jump_table = JumpTable::default();
                // subsecond::run_patch(so, jump_table);
                // dioxus_core::prelude::force_all_dirty();
            }
        }
    });
}

/// Connect to the devserver and handle its messages with a callback.
///
/// This doesn't use any form of security or protocol, so it's not safe to expose to the internet.
#[cfg(not(target_arch = "wasm32"))]
pub fn connect(endpoint: String, mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    std::thread::spawn(move || {
        let (mut websocket, _req) = match tungstenite::connect(endpoint.clone()) {
            Ok((websocket, req)) => (websocket, req),
            Err(_) => return,
        };

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Text(text) = msg {
                if let Ok(msg) = serde_json::from_str(&text) {
                    callback(msg);
                }
            }
        }
    });
}
