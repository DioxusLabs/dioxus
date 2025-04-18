use dioxus_core::{ScopeId, VirtualDom};
use dioxus_signals::{GlobalKey, Writable};
use warnings::Warning;

pub use dioxus_devtools_types::*;
pub use subsecond;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) -> Result<(), subsecond::PatchError> {
    dom.runtime().on_scope(ScopeId::ROOT, || {
        // 1. Update signals...
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

        // 2. Attempt to hotpatch
        if let Some(jump_table) = msg.jump_table.as_ref().cloned() {
            unsafe { subsecond::apply_patch(jump_table) }?;
            dioxus_core::prelude::force_all_dirty();
        }

        Ok(())
    })
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

        _ = websocket.send(tungstenite::Message::Text(
            serde_json::to_string(&ClientMsg::Initialize {
                aslr_reference: subsecond::aslr_reference() as _,
                build_id: dioxus_cli_config::build_id(),
            })
            .unwrap()
            .into(),
        ));

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Text(text) = msg {
                if let Ok(msg) = serde_json::from_str(&text) {
                    callback(msg);
                }
            }
        }
    });
}
