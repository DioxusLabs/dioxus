use crate::HotReloadMsg;
use dioxus_core::{internal::HotReloadLiteral, ScopeId, VirtualDom};
use dioxus_signals::Writable;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &mut VirtualDom, msg: &HotReloadMsg) {
    for templates in &msg.templates {
        for template in &templates.templates {
            dom.replace_template(*template);
        }

        dom.runtime().on_scope(ScopeId::ROOT, || {
            let ctx = dioxus_signals::get_global_context();

            for (id, literal) in templates.changed_lits.iter() {
                match &literal {
                    HotReloadLiteral::Fmted(f) => {
                        if let Some(mut signal) = ctx.get_signal_with_key(id) {
                            signal.set(f.clone());
                        }
                    }
                    HotReloadLiteral::Float(f) => {
                        if let Some(mut signal) = ctx.get_signal_with_key::<f64>(id) {
                            signal.set(*f);
                        }
                    }
                    HotReloadLiteral::Int(f) => {
                        if let Some(mut signal) = ctx.get_signal_with_key::<i64>(id) {
                            signal.set(*f);
                        }
                    }
                    HotReloadLiteral::Bool(f) => {
                        if let Some(mut signal) = ctx.get_signal_with_key::<bool>(id) {
                            signal.set(*f);
                        }
                    }
                }
            }
        });
    }
}
