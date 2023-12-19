use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeState, TemplateNode, VNode, VirtualDom,
};
use dioxus_html::event_bubbles;
use wasm_bindgen::JsCast;
use web_sys::{Comment, Node};

impl WebsysDom {
    // we're streaming in patches, but the nodes already exist
    // so we're just going to write the correct IDs to the node and load them in
    pub fn rehydrate(&mut self) {
        dioxus_interpreter_js::hydrate()
    }
}
