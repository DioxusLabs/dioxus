use std::fmt::Write;

use dioxus_core::{LazyNodes, ScopeId, VirtualDom};

use crate::config::SsrConfig;

pub fn pre_render(dom: &VirtualDom) -> String {
    todo!()
}

pub fn pre_render_to(dom: &VirtualDom, write: impl Write) {
    todo!()
}

pub fn render_vdom(dom: &VirtualDom) -> String {
    todo!()
    // format!("{:}", TextRenderer::from_vdom(dom, SsrConfig::default()))
}

pub fn pre_render_vdom(dom: &VirtualDom) -> String {
    todo!()
    // format!(
    //     "{:}",
    //     TextRenderer::from_vdom(dom, SsrConfig::default().pre_render(true))
    // )
}

pub fn render_vdom_cfg(dom: &VirtualDom, cfg: SsrConfig) -> String {
    todo!()
    // format!(
    //     "{:}",
    //     TextRenderer::from_vdom(dom, cfg(SsrConfig::default()))
    // )
}

pub fn render_vdom_scope(vdom: &VirtualDom, scope: ScopeId) -> Option<String> {
    todo!()
    // Some(format!(
    //     "{:}",
    //     TextRenderer {
    //         cfg: SsrConfig::default(),
    //         root: vdom.get_scope(scope).unwrap().root_node(),
    //         vdom: Some(vdom),
    //     }
    // ))
}

pub fn render_lazy<'a, 'b>(f: LazyNodes<'a, 'b>) -> String {
    todo!()
}
