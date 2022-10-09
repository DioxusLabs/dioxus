use crate::dom::WebsysDom;
use dioxus_core::{VNode, VirtualDom};
use dioxus_html::event_bubbles;
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

                self.interpreter.SetNode(node_id.0, node);

                *cur_place += 1;
            }

            VNode::Element(vel) => {
                let node_id = vel.id.get().ok_or(VNodeNotInitialized)?;

                let cur_place = place.last_mut().unwrap();

                let node = nodes.last().unwrap().child_nodes().get(*cur_place).unwrap();

                self.interpreter.SetNode(node_id.0, node.clone());

                *cur_place += 1;

                nodes.push(node.clone());

                place.push(0);

                // we cant have the last node be text
                let mut last_node_was_text = false;
                for child in vel.children {
                    self.rehydrate_single(nodes, place, dom, child, &mut last_node_was_text)?;
                }

                for listener in vel.listeners {
                    let global_id = listener.mounted_node.get().unwrap();
                    match global_id {
                        dioxus_core::GlobalNodeId::TemplateId {
                            template_ref_id,
                            template_node_id: id,
                        } => {
                            self.interpreter.EnterTemplateRef(template_ref_id.into());
                            self.interpreter.NewEventListener(
                                listener.event,
                                id.into(),
                                self.handler.as_ref().unchecked_ref(),
                                event_bubbles(listener.event),
                            );
                            self.interpreter.ExitTemplateRef();
                        }
                        dioxus_core::GlobalNodeId::VNodeId(id) => {
                            self.interpreter.NewEventListener(
                                listener.event,
                                id.into(),
                                self.handler.as_ref().unchecked_ref(),
                                event_bubbles(listener.event),
                            );
                        }
                    }
                }

                if !vel.listeners.is_empty() {
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

                self.interpreter.SetNode(node_id.0, node);

                // self.nodes[node_id.0] = Some(node);

                *cur_place += 1;
            }

            VNode::Fragment(el) => {
                for el in el.children {
                    self.rehydrate_single(nodes, place, dom, el, last_node_was_text)?;
                }
            }

            VNode::Component(el) => {
                let scope = dom.get_scope(el.scope.get().unwrap()).unwrap();
                let node = scope.root_node();
                self.rehydrate_single(nodes, place, dom, node, last_node_was_text)?;
            }
            VNode::TemplateRef(_) => todo!(),
        }
        Ok(())
    }
}
