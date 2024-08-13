use crate::HotReloadMsg;
use dioxus_core::{ScopeId, VirtualDom};
use dioxus_signals::Writable;

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
                signal.set(Some(value));
            }
        }
    });
}
