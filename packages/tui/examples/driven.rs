use dioxus_html::EventData;
use dioxus_native_core::{
    node::{OwnedAttributeDiscription, OwnedAttributeValue, TextNode},
    prelude::*,
    real_dom::{NodeImmutable, NodeTypeMut},
    NodeId,
};
use dioxus_tui::{self, render, Config, Renderer};
use rustc_hash::FxHashSet;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

const SIZE: usize = 10;

struct Test {
    node_states: [[usize; SIZE]; SIZE],
    dirty: FxHashSet<(usize, usize)>,
}

impl Default for Test {
    fn default() -> Self {
        Self {
            node_states: [[0; SIZE]; SIZE],
            dirty: FxHashSet::default(),
        }
    }
}

impl Test {
    fn create(mut root: NodeMut) -> Self {
        let myself = Self::default();

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
        let rdom = root.real_dom_mut();

        // create the grid
        for (x, row) in myself.node_states.iter().copied().enumerate() {
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
        myself
    }
}

impl Renderer for Test {
    fn render(&mut self, rdom: &Arc<RwLock<RealDom>>) {
        let mut rdom = rdom.write().unwrap();
        let root_id = rdom.root_id();
        let mut root = rdom.get_mut(root_id).unwrap();
        for (x, y) in self.dirty.drain() {
            let row_id = root.child_ids().unwrap()[x];
            let rdom = root.real_dom_mut();
            let row = rdom.get(row_id).unwrap();
            let node_id = row.child_ids().unwrap()[y];
            let mut node = rdom.get_mut(node_id).unwrap();
            if let NodeTypeMut::Element(mut el) = node.node_type_mut() {
                el.set_attribute(
                    OwnedAttributeDiscription {
                        name: "background-color".into(),
                        namespace: None,
                    },
                    OwnedAttributeValue::Text(format!(
                        "rgb({}, {}, {})",
                        self.node_states[x][y] * 10,
                        0,
                        (x + y) * 10,
                    )),
                );
            }
            let text_id = *node.child_ids().unwrap().first().unwrap();
            let mut text = rdom.get_mut(text_id).unwrap();
            if let NodeTypeMut::Text(text) = text.node_type_mut() {
                *text = self.node_states[x][y].to_string();
            }
        }
    }

    fn handle_event(
        &mut self,
        rdom: &Arc<RwLock<RealDom>>,
        id: NodeId,
        _: &str,
        _: Rc<EventData>,
        _: bool,
    ) {
        let rdom = rdom.read().unwrap();
        let node = rdom.get(id).unwrap();
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
                self.node_states[parents_child_number][child_number] += 1;
                self.dirty.insert((parents_child_number, child_number));
            }
        }
    }

    fn poll_async(&mut self) -> std::pin::Pin<Box<dyn futures::Future<Output = ()> + '_>> {
        Box::pin(async move { tokio::time::sleep(std::time::Duration::from_millis(1000)).await })
    }
}

fn main() {
    render(Config::new(), |rdom, _, _| {
        let mut rdom = rdom.write().unwrap();
        let root = rdom.root_id();
        Test::create(rdom.get_mut(root).unwrap())
    })
    .unwrap();
}
