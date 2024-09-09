use crate::HotReloadMsg;
use dioxus_core::{ScopeId, VirtualDom};
use dioxus_signals::Writable;
use warnings::Warning;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &mut VirtualDom, msg: &HotReloadMsg) {
    dom.runtime().on_scope(ScopeId::ROOT, || {
        let ctx = dioxus_signals::get_global_context();

        for template in &msg.templates {
            let id = &template.location;
            let value = template.template.clone();
            if let Some(mut signal) = ctx.get_signal_with_key(id) {
                dioxus_signals::warnings::signal_read_and_write_in_reactive_scope::allow(|| {
                    dioxus_signals::warnings::signal_write_in_component_body::allow(|| {
                        signal.set(Some(value));
                    });
                });
            }
        }
    });
}
