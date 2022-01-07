use crate::dom::WebsysDom;
use dioxus_core::{VNode, VirtualDom};
use wasm_bindgen::JsCast;
use web_sys::{Comment, Element, Node, Text};

#[derive(Debug)]
pub enum RehydrationError {
    NodeTypeMismatch,
    NodeNotFound,
    VNodeNotInitialized,
}
use RehydrationError::*;

impl WebsysDom {
    // we're streaming in patches, but the nodes already exist
    // so we're just going to write the correct IDs to the node and load them in
    pub fn rehydrate(&mut self, dom: &VirtualDom) -> Result<(), RehydrationError> {
        let root = self
            .root
            .clone()
            .dyn_into::<Node>()
            .map_err(|_| NodeTypeMismatch)?;

        let root_scope = dom.base_scope();
        let root_node = root_scope.root_node();

        let mut nodes = vec![root];
        let mut counter = vec![0];

        let mut last_node_was_text = false;

        // Recursively rehydrate the dom from the VirtualDom
        self.rehydrate_single(
            &mut nodes,
            &mut counter,
            dom,
            root_node,
            &mut last_node_was_text,
        )
    }

    fn rehydrate_single(
        &mut self,
        nodes: &mut Vec<Node>,
        place: &mut Vec<u32>,
        dom: &VirtualDom,
        node: &VNode,
        last_node_was_text: &mut bool,
    ) -> Result<(), RehydrationError> {
        match node {
            VNode::Text(t) => {
                let node_id = t.id.get().ok_or(VNodeNotInitialized)?;

                let cur_place = place.last_mut().unwrap();

                // skip over the comment element
                if *last_node_was_text {
                    if cfg!(debug_assertions) {
                        let node = nodes.last().unwrap().child_nodes().get(*cur_place).unwrap();
                        let node_text = node.dyn_into::<Comment>().unwrap();
                        assert_eq!(node_text.data(), "spacer");
                    }
                    *cur_place += 1;
                }

                let node = nodes
                    .last()
                    .unwrap()
                    .child_nodes()
                    .get(*cur_place)
                    .ok_or(NodeNotFound)?;

                let _text_el = node.dyn_ref::<Text>().ok_or(NodeTypeMismatch)?;

                // in debug we make sure the text is the same
                if cfg!(debug_assertions) {
                    let contents = _text_el.node_value().unwrap();
                    assert_eq!(t.text, contents);
                }

                *last_node_was_text = true;

                self.nodes[node_id.0] = Some(node);

                *cur_place += 1;
            }

            VNode::Element(vel) => {
                let node_id = vel.id.get().ok_or(VNodeNotInitialized)?;

                let cur_place = place.last_mut().unwrap();

                let node = nodes.last().unwrap().child_nodes().get(*cur_place).unwrap();

                use smallstr::SmallString;
                use std::fmt::Write;

                // 8 digits is enough, yes?
                // 12 million nodes in one page?
                let mut s: SmallString<[u8; 8]> = smallstr::SmallString::new();
                write!(s, "{}", node_id).unwrap();

                node.dyn_ref::<Element>()
                    .unwrap()
                    .set_attribute("dioxus-id", s.as_str())
                    .unwrap();

                self.nodes[node_id.0] = Some(node.clone());

                *cur_place += 1;

                nodes.push(node.clone());

                place.push(0);

                // we cant have the last node be text
                let mut last_node_was_text = false;
                for child in vel.children {
                    self.rehydrate_single(nodes, place, dom, &child, &mut last_node_was_text)?;
                }

                place.pop();
                nodes.pop();

                if cfg!(debug_assertions) {
                    let el = node.dyn_ref::<Element>().unwrap();
                    let name = el.tag_name().to_lowercase();
                    assert_eq!(name, vel.tag);
                }
            }

            VNode::Placeholder(el) => {
                let node_id = el.id.get().ok_or(VNodeNotInitialized)?;

                let cur_place = place.last_mut().unwrap();
                let node = nodes.last().unwrap().child_nodes().get(*cur_place).unwrap();

                self.nodes[node_id.0] = Some(node);

                *cur_place += 1;
            }

            VNode::Fragment(el) => {
                for el in el.children {
                    self.rehydrate_single(nodes, place, dom, &el, last_node_was_text)?;
                }
            }

            VNode::Component(el) => {
                let scope = dom.get_scope(el.scope.get().unwrap()).unwrap();
                let node = scope.root_node();
                self.rehydrate_single(nodes, place, dom, node, last_node_was_text)?;
            }
        }
        Ok(())
    }
}
