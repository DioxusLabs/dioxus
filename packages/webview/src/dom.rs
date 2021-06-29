//! webview dom

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core::{
    diff::RealDom,
    prelude::ScopeIdx,
    virtual_dom::{RealDomNode, VirtualDom},
};
use serde::{Deserialize, Serialize};

fn test() {
    const App: FC<()> = |cx| cx.render(rsx! { div {}});
    let mut vi = VirtualDom::new(App);
    let mut real = WebviewDom {
        edits: Vec::new(),
        node_counter: 0,
    };
    vi.rebuild(&mut real);
}

pub struct WebviewDom<'bump> {
    pub edits: Vec<SerializedDom<'bump>>,
    pub node_counter: u64,
}
impl WebviewDom<'_> {
    pub fn new() -> Self {
        Self {
            edits: Vec::new(),
            node_counter: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SerializedDom<'bump> {
    PushRoot {
        root: u64,
    },
    AppendChild,
    ReplaceWith,
    Remove,
    RemoveAllChildren,
    CreateTextNode {
        text: &'bump str,
        id: u64,
    },
    CreateElement {
        tag: &'bump str,
        id: u64,
    },
    CreateElementNs {
        tag: &'bump str,
        id: u64,
        ns: &'bump str,
    },
    CreatePlaceholder {
        id: u64,
    },
    NewEventListener {
        event: &'bump str,
        scope: ScopeIdx,
        node: u64,
        idx: usize,
    },
    RemoveEventListener {
        event: &'bump str,
    },
    SetText {
        text: &'bump str,
    },
    SetAttribute {
        field: &'bump str,
        value: &'bump str,
        ns: Option<&'bump str>,
    },
    RemoveAttribute {
        name: &'bump str,
    },
}

use SerializedDom::*;
impl<'bump> RealDom<'bump> for WebviewDom<'bump> {
    fn push_root(&mut self, root: dioxus_core::virtual_dom::RealDomNode) {
        self.edits.push(PushRoot { root: root.0 });
    }

    fn append_child(&mut self) {
        self.edits.push(AppendChild);
    }

    fn replace_with(&mut self) {
        self.edits.push(ReplaceWith);
    }

    fn remove(&mut self) {
        self.edits.push(Remove);
    }

    fn remove_all_children(&mut self) {
        self.edits.push(RemoveAllChildren);
    }

    fn create_text_node(&mut self, text: &'bump str) -> dioxus_core::virtual_dom::RealDomNode {
        self.node_counter += 1;
        let id = RealDomNode::new(self.node_counter);
        self.edits.push(CreateTextNode { text, id: id.0 });
        id
    }

    fn create_element(
        &mut self,
        tag: &'bump str,
        ns: Option<&'bump str>,
    ) -> dioxus_core::virtual_dom::RealDomNode {
        self.node_counter += 1;
        let id = RealDomNode::new(self.node_counter);
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { id: id.0, ns, tag }),
            None => self.edits.push(CreateElement { id: id.0, tag }),
        }
        id
    }

    fn create_placeholder(&mut self) -> dioxus_core::virtual_dom::RealDomNode {
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
        realnode: dioxus_core::virtual_dom::RealDomNode,
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
