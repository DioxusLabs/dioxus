use dioxus_html::EventData;
use dioxus_native_core::{
    node::{OwnedAttributeDiscription, OwnedAttributeValue, TextNode},
    prelude::*,
    real_dom::{NodeImmutable, NodeTypeMut},
    NodeId, Renderer,
};
use dioxus_tui::{self, render, Config};
use std::sync::{Arc, RwLock};
use std::{rc::Rc, sync::Mutex};
use taffy::Taffy;

struct Test([[usize; 10]; 10]);

impl Renderer<Rc<EventData>> for Test {
    fn render(&mut self, mut root: dioxus_native_core::NodeMut) {
        // Set the root node to be a flexbox with a column direction.
        if let NodeTypeMut::Element(mut el) = root.node_type_mut() {
            el.set_attribute(
                OwnedAttributeDiscription {
                    name: "display".into(),
                    namespace: None,
                },
                OwnedAttributeValue::Text("flex".into()),
            );
            el.set_attribute(
                OwnedAttributeDiscription {
                    name: "flex-direction".into(),
                    namespace: None,
                },
                OwnedAttributeValue::Text("column".into()),
            );
            el.set_attribute(
                OwnedAttributeDiscription {
                    name: "width".into(),
                    namespace: None,
                },
                OwnedAttributeValue::Text("100%".into()),
            );
            el.set_attribute(
                OwnedAttributeDiscription {
                    name: "height".into(),
                    namespace: None,
                },
                OwnedAttributeValue::Text("100%".into()),
            );
        }

        let root_id = root.id();
        // Remove old grid. Frameworks should retain the grid and only update the values.
        let children_ids = root.child_ids().map(|ids| ids.to_vec());
        let rdom = root.real_dom_mut();
        if let Some(children) = children_ids {
            for child in children {
                rdom.get_mut(child).unwrap().remove();
            }
        }

        // create the grid
        for (x, row) in self.0.iter().copied().enumerate() {
            let row_node = rdom
                .create_node(NodeType::Element(ElementNode {
                    tag: "div".to_string(),
                    attributes: [
                        (
                            OwnedAttributeDiscription {
                                name: "display".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("flex".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "flex-direction".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("row".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "width".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("100%".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "height".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("100%".into()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }))
                .id();
            for (y, count) in row.iter().copied().enumerate() {
                let node = rdom
                    .create_node(NodeType::Text(TextNode::new(count.to_string())))
                    .id();
                let mut button = rdom.create_node(NodeType::Element(ElementNode {
                    tag: "div".to_string(),
                    attributes: [
                        (
                            OwnedAttributeDiscription {
                                name: "background-color".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text(format!(
                                "rgb({}, {}, {})",
                                count * 10,
                                0,
                                (x + y) * 10,
                            )),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "width".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("100%".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "height".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("100%".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "display".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("flex".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "flex-direction".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("row".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "justify-content".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("center".into()),
                        ),
                        (
                            OwnedAttributeDiscription {
                                name: "align-items".into(),
                                namespace: None,
                            },
                            OwnedAttributeValue::Text("center".into()),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }));
                button.add_event_listener("click");
                button.add_event_listener("wheel");
                button.add_child(node);
                let button_id = button.id();
                rdom.get_mut(row_node).unwrap().add_child(button_id);
            }
            rdom.get_mut(root_id).unwrap().add_child(row_node);
        }
    }

    fn handle_event(
        &mut self,
        node: dioxus_native_core::NodeMut<()>,
        event: &str,
        value: Rc<EventData>,
        bubbles: bool,
    ) {
        if let Some(parent) = node.parent() {
            let child_number = parent
                .child_ids()
                .unwrap()
                .iter()
                .position(|id| *id == node.id())
                .unwrap();
            if let Some(parents_parent) = parent.parent() {
                let parents_child_number = parents_parent
                    .child_ids()
                    .unwrap()
                    .iter()
                    .position(|id| *id == parent.id())
                    .unwrap();
                self.0[parents_child_number][child_number] += 1;
            }
        }
    }

    fn poll_async(&mut self) -> std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>> {
        Box::pin(async move { tokio::time::sleep(std::time::Duration::from_millis(1000)).await })
    }
}

fn main() {
    render(Config::new(), |_, _, _| Test(Default::default())).unwrap();
}
