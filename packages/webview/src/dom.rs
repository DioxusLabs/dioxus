//! webview dom

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core::{diff::RealDom, serialize::DomEdit, virtual_dom::VirtualDom};
use DomEdit::*;

pub struct WebviewRegistry {}

impl WebviewRegistry {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct WebviewDom<'bump> {
    pub edits: Vec<DomEdit<'bump>>,
    pub node_counter: u64,
    pub registry: WebviewRegistry,
}
impl WebviewDom<'_> {
    pub fn new(registry: WebviewRegistry) -> Self {
        Self {
            edits: Vec::new(),
            node_counter: 0,
            registry,
        }
    }

    // Finish using the dom (for its edit list) and give back the node and event registry
    pub fn consume(self) -> WebviewRegistry {
        self.registry
    }
}
impl<'bump> RealDom<'bump> for WebviewDom<'bump> {
    fn push_root(&mut self, root: RealDomNode) {
        self.edits.push(PushRoot { root: root.0 });
    }

    fn append_children(&mut self, many: u32) {
        self.edits.push(AppendChild);
    }

    fn replace_with(&mut self, many: u32) {
        self.edits.push(ReplaceWith);
    }

    fn remove(&mut self) {
        self.edits.push(Remove);
    }

    fn remove_all_children(&mut self) {
        self.edits.push(RemoveAllChildren);
    }

    fn create_text_node(&mut self, text: &'bump str) -> RealDomNode {
        self.node_counter += 1;
        let id = RealDomNode::new(self.node_counter);
        self.edits.push(CreateTextNode { text, id: id.0 });
        id
    }

    fn create_element(&mut self, tag: &'bump str, ns: Option<&'bump str>) -> RealDomNode {
        self.node_counter += 1;
        let id = RealDomNode::new(self.node_counter);
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { id: id.0, ns, tag }),
            None => self.edits.push(CreateElement { id: id.0, tag }),
        }
        id
    }

    fn create_placeholder(&mut self) -> RealDomNode {
        self.node_counter += 1;
        let id = RealDomNode::new(self.node_counter);
        self.edits.push(CreatePlaceholder { id: id.0 });
        id
    }

    fn new_event_listener(
        &mut self,
        event: &'static str,
        scope: dioxus_core::prelude::ScopeIdx,
        element_id: usize,
        realnode: RealDomNode,
    ) {
        self.edits.push(NewEventListener {
            scope,
            event,
            idx: element_id,
            node: realnode.0,
        });
    }

    fn remove_event_listener(&mut self, event: &'static str) {
        self.edits.push(RemoveEventListener { event });
    }

    fn set_text(&mut self, text: &'bump str) {
        self.edits.push(SetText { text });
    }

    fn set_attribute(&mut self, field: &'static str, value: &'bump str, ns: Option<&'bump str>) {
        self.edits.push(SetAttribute { field, value, ns });
    }

    fn remove_attribute(&mut self, name: &'static str) {
        self.edits.push(RemoveAttribute { name });
    }

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        todo!()
        // self.edits.push(PushRoot { root });
    }
}
