use std::collections::HashMap;

use dioxus_core::virtual_dom::RealDomNode;
use nohash_hasher::IntMap;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    window, Document, Element, Event, HtmlElement, HtmlInputElement, HtmlOptionElement, Node,
};

use crate::interpreter::Stack;
pub struct WebsysDom {
    pub stack: Stack,
    nodes: IntMap<u32, Node>,
    document: Document,
    root: Element,

    // We need to make sure to add comments between text nodes
    // We ensure that the text siblings are patched by preventing the browser from merging
    // neighboring text nodes. Originally inspired by some of React's work from 2016.
    //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
    //  -> https://github.com/facebook/react/pull/5753
    //
    // `ptns` = Percy text node separator
    // TODO
    last_node_was_text: bool,

    node_counter: Counter,
}
impl WebsysDom {
    pub fn new(root: Element) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        Self {
            stack: Stack::with_capacity(10),
            nodes: HashMap::with_capacity_and_hasher(
                1000,
                nohash_hasher::BuildNoHashHasher::default(),
            ),
            document,
            root,
            last_node_was_text: false,
            node_counter: Counter(0),
        }
    }
}

struct Counter(u32);
impl Counter {
    fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}
impl dioxus_core::diff::RealDom for WebsysDom {
    fn push_root(&mut self, root: dioxus_core::virtual_dom::RealDomNode) {
        let domnode = self.nodes.get(&root.0).expect("Failed to pop know root");
        self.stack.push(domnode.clone());
    }

    fn append_child(&mut self) {
        let child = self.stack.pop();

        if child.dyn_ref::<web_sys::Text>().is_some() {
            if self.last_node_was_text {
                let comment_node = self
                    .document
                    .create_comment("dioxus")
                    .dyn_into::<Node>()
                    .unwrap();
                self.stack.top().append_child(&comment_node).unwrap();
            }
            self.last_node_was_text = true;
        } else {
            self.last_node_was_text = false;
        }

        self.stack.top().append_child(&child).unwrap();
    }

    fn replace_with(&mut self) {
        let new_node = self.stack.pop();
        let old_node = self.stack.pop();

        if old_node.has_type::<Element>() {
            old_node
                .dyn_ref::<Element>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else if old_node.has_type::<web_sys::CharacterData>() {
            old_node
                .dyn_ref::<web_sys::CharacterData>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else if old_node.has_type::<web_sys::DocumentType>() {
            old_node
                .dyn_ref::<web_sys::DocumentType>()
                .unwrap()
                .replace_with_with_node_1(&new_node)
                .unwrap();
        } else {
            panic!("Cannot replace node: {:?}", old_node);
        }

        // // poc to see if this is a valid solution
        // if let Some(id) = self.current_known {
        //     // update mapping
        //     self.known_roots.insert(id, new_node.clone());
        //     self.current_known = None;
        // }

        self.stack.push(new_node);
    }

    fn remove(&mut self) {
        todo!()
    }

    fn remove_all_children(&mut self) {
        todo!()
    }

    fn create_text_node(&mut self, text: &str) -> dioxus_core::virtual_dom::RealDomNode {
        let nid = self.node_counter.next();
        let textnode = self
            .document
            .create_text_node(text)
            .dyn_into::<Node>()
            .unwrap();
        self.stack.push(textnode.clone());
        self.nodes.insert(nid, textnode);

        RealDomNode::new(nid)
    }

    fn create_element(&mut self, tag: &str) -> dioxus_core::virtual_dom::RealDomNode {
        let el = self
            .document
            .create_element(tag)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();

        self.stack.push(el.clone());
        let nid = self.node_counter.next();
        self.nodes.insert(nid, el);
        RealDomNode::new(nid)
    }

    fn create_element_ns(
        &mut self,
        tag: &str,
        namespace: &str,
    ) -> dioxus_core::virtual_dom::RealDomNode {
        let el = self
            .document
            .create_element_ns(Some(namespace), tag)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();

        self.stack.push(el.clone());
        let nid = self.node_counter.next();
        self.nodes.insert(nid, el);
        RealDomNode::new(nid)
    }

    fn new_event_listener(
        &mut self,
        event: &str,
        scope: dioxus_core::prelude::ScopeIdx,
        id: usize,
    ) {
        // if let Some(entry) = self.listeners.get_mut(event) {
        //     entry.0 += 1;
        // } else {
        //     let trigger = self.trigger.clone();
        //     let handler = Closure::wrap(Box::new(move |event: &web_sys::Event| {
        //         log::debug!("Handling event!");

        //         let target = event
        //             .target()
        //             .expect("missing target")
        //             .dyn_into::<Element>()
        //             .expect("not a valid element");

        //         let typ = event.type_();

        //         let gi_id: Option<usize> = target
        //             .get_attribute(&format!("dioxus-giid-{}", typ))
        //             .and_then(|v| v.parse().ok());

        //         let gi_gen: Option<u64> = target
        //             .get_attribute(&format!("dioxus-gigen-{}", typ))
        //             .and_then(|v| v.parse().ok());

        //         let li_idx: Option<usize> = target
        //             .get_attribute(&format!("dioxus-lidx-{}", typ))
        //             .and_then(|v| v.parse().ok());

        //         if let (Some(gi_id), Some(gi_gen), Some(li_idx)) = (gi_id, gi_gen, li_idx) {
        //             // Call the trigger
        //             log::debug!(
        //                 "decoded gi_id: {},  gi_gen: {},  li_idx: {}",
        //                 gi_id,
        //                 gi_gen,
        //                 li_idx
        //             );

        //             let triggered_scope = ScopeIdx::from_raw_parts(gi_id, gi_gen);
        //             trigger.0.as_ref()(EventTrigger::new(
        //                 virtual_event_from_websys_event(event),
        //                 triggered_scope,
        //                 // scope,
        //                 li_idx,
        //             ));
        //         }
        //     }) as Box<dyn FnMut(&Event)>);

        //     self.root
        //         .add_event_listener_with_callback(event, (&handler).as_ref().unchecked_ref())
        //         .unwrap();

        //     // Increment the listeners
        //     self.listeners.insert(event.into(), (1, handler));
        // }
    }

    fn remove_event_listener(&mut self, event: &str) {
        todo!()
    }

    fn set_text(&mut self, text: &str) {
        self.stack.top().set_text_content(Some(text))
    }

    fn set_attribute(&mut self, name: &str, value: &str, is_namespaced: bool) {
        if name == "class" {
            if let Some(el) = self.stack.top().dyn_ref::<Element>() {
                el.set_class_name(value);
            }
        } else {
        }
    }

    fn remove_attribute(&mut self, name: &str) {
        let node = self.stack.top();
        if let Some(node) = node.dyn_ref::<web_sys::Element>() {
            node.remove_attribute(name).unwrap();
        }
        if let Some(node) = node.dyn_ref::<HtmlInputElement>() {
            // Some attributes are "volatile" and don't work through `removeAttribute`.
            if name == "value" {
                node.set_value("");
            }
            if name == "checked" {
                node.set_checked(false);
            }
        }

        if let Some(node) = node.dyn_ref::<HtmlOptionElement>() {
            if name == "selected" {
                node.set_selected(true);
            }
        }
    }

    fn raw_node_as_any_mut(&self) -> &mut dyn std::any::Any {
        todo!()
    }
}
