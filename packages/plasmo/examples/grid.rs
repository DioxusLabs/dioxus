use dioxus_native_core::{
    node::TextNode,
    prelude::*,
    real_dom::{NodeImmutable, NodeTypeMut},
    NodeId,
};
use plasmo::{render, Config, Driver, EventData};
use rustc_hash::FxHashSet;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

const SIZE: usize = 20;

#[derive(Default, Clone, Copy)]
struct Node {
    container_id: Option<NodeId>,
    text_id: Option<NodeId>,
    count: usize,
}

struct Test {
    node_states: [[Node; SIZE]; SIZE],
    dirty: FxHashSet<(usize, usize)>,
}

impl Default for Test {
    fn default() -> Self {
        Self {
            node_states: [[Node {
                container_id: None,
                text_id: None,
                count: 0,
            }; SIZE]; SIZE],
            dirty: FxHashSet::default(),
        }
    }
}

impl Test {
    fn create(mut root: NodeMut) -> Self {
        let mut myself = Self::default();

        // Set the root node to be a flexbox with a column direction.
        if let NodeTypeMut::Element(mut el) = root.node_type_mut() {
            el.set_attribute("display".to_string(), "flex".to_string());
            el.set_attribute(("flex-direction", "style"), "column".to_string());
            el.set_attribute(("width", "style"), "100%".to_string());
            el.set_attribute(("height", "style"), "100%".to_string());
        }

        let root_id = root.id();
        let rdom = root.real_dom_mut();

        // create the grid
        for (x, row) in myself.node_states.iter_mut().enumerate() {
            let row_node = rdom
                .create_node(NodeType::Element(ElementNode {
                    tag: "div".to_string(),
                    attributes: [
                        ("display".to_string().into(), "flex".to_string().into()),
                        (("flex-direction", "style").into(), "row".to_string().into()),
                        (("width", "style").into(), "100%".to_string().into()),
                        (("height", "style").into(), "100%".to_string().into()),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }))
                .id();
            for (y, node) in row.iter_mut().enumerate() {
                let count = node.count;
                let id = rdom
                    .create_node(NodeType::Text(TextNode::new(count.to_string())))
                    .id();
                let mut button = rdom.create_node(NodeType::Element(ElementNode {
                    tag: "div".to_string(),
                    attributes: [
                        ("display".to_string().into(), "flex".to_string().into()),
                        (
                            ("background-color", "style").into(),
                            format!("rgb({}, {}, {})", count * 10, 0, (x + y),).into(),
                        ),
                        (("width", "style").into(), "100%".to_string().into()),
                        (("height", "style").into(), "100%".to_string().into()),
                        (("flex-direction", "style").into(), "row".to_string().into()),
                        (
                            ("justify-content", "style").into(),
                            "center".to_string().into(),
                        ),
                        (("align-items", "style").into(), "center".to_string().into()),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }));
                button.add_event_listener("click");
                button.add_event_listener("wheel");
                button.add_child(id);
                let button_id = button.id();
                rdom.get_mut(row_node).unwrap().add_child(button_id);
                node.container_id = Some(button_id);
                node.text_id = Some(id);
            }
            rdom.get_mut(root_id).unwrap().add_child(row_node);
        }
        myself
    }
}

impl Driver for Test {
    fn update(&mut self, rdom: &Arc<RwLock<RealDom>>) {
        let mut rdom = rdom.write().unwrap();
        for (x, y) in self.dirty.drain() {
            let node = self.node_states[x][y];
            let node_id = node.container_id.unwrap();
            let mut container = rdom.get_mut(node_id).unwrap();
            if let NodeTypeMut::Element(mut el) = container.node_type_mut() {
                el.set_attribute(
                    ("background-color", "style"),
                    format!("rgb({}, {}, {})", node.count * 10, 0, (x + y),),
                );
            }
            let text_id = node.text_id.unwrap();
            let mut text = rdom.get_mut(text_id).unwrap();
            let type_mut = text.node_type_mut();
            if let NodeTypeMut::Text(mut text) = type_mut {
                *text = node.count.to_string();
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
                .iter()
                .position(|id| *id == node.id())
                .unwrap();
            if let Some(parents_parent) = parent.parent() {
                let parents_child_number = parents_parent
                    .child_ids()
                    .iter()
                    .position(|id| *id == parent.id())
                    .unwrap();
                self.node_states[parents_child_number][child_number].count += 1;
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
