use dioxus_native_core::{
    node::TextNode,
    prelude::*,
    real_dom::{NodeImmutable, NodeTypeMut},
    NodeId,
};
use plasmo::{render, Config, Driver, EventData};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(Default)]
struct Counter {
    count: usize,
    counter_id: NodeId,
    button_id: NodeId,
}

impl Counter {
    fn create(mut root: NodeMut) -> Self {
        let mut myself = Self::default();

        let root_id = root.id();
        let rdom = root.real_dom_mut();

        // create the counter
        let count = myself.count;
        myself.counter_id = rdom
            .create_node(NodeType::Text(TextNode::new(count.to_string())))
            .id();
        let mut button = rdom.create_node(NodeType::Element(ElementNode {
            tag: "div".to_string(),
            attributes: [
                ("display".to_string().into(), "flex".to_string().into()),
                (
                    ("background-color", "style").into(),
                    format!("rgb({}, {}, {})", count * 10, 0, 0,).into(),
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
        button.add_child(myself.counter_id);
        myself.button_id = button.id();
        rdom.get_mut(root_id).unwrap().add_child(myself.button_id);

        myself
    }
}

impl Driver for Counter {
    fn update(&mut self, rdom: &Arc<RwLock<RealDom>>) {
        // update the counter
        let mut rdom = rdom.write().unwrap();
        let mut node = rdom.get_mut(self.button_id).unwrap();
        if let NodeTypeMut::Element(mut el) = node.node_type_mut() {
            el.set_attribute(
                ("background-color", "style"),
                format!("rgb({}, {}, {})", self.count * 10, 0, 0,),
            );
        }
        let mut text = rdom.get_mut(self.counter_id).unwrap();
        let type_mut = text.node_type_mut();
        if let NodeTypeMut::Text(mut text) = type_mut {
            *text = self.count.to_string();
        }
    }

    fn handle_event(
        &mut self,
        _: &Arc<RwLock<RealDom>>,
        _: NodeId,
        _: &str,
        _: Rc<EventData>,
        _: bool,
    ) {
        // when a click or wheel event is fired, increment the counter
        self.count += 1;
    }

    fn poll_async(&mut self) -> std::pin::Pin<Box<dyn futures::Future<Output = ()> + '_>> {
        Box::pin(async move { tokio::time::sleep(std::time::Duration::from_millis(1000)).await })
    }
}

fn main() {
    render(Config::new(), |rdom, _, _| {
        let mut rdom = rdom.write().unwrap();
        let root = rdom.root_id();
        Counter::create(rdom.get_mut(root).unwrap())
    })
    .unwrap();
}
