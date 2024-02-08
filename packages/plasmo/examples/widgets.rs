use dioxus_html::HasFormData;
use dioxus_native_core::{
    prelude::*,
    real_dom::{NodeImmutable, NodeTypeMut},
    NodeId,
};
use plasmo::{render, Config, Driver, EventData};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(Default)]
struct Counter {
    count: f64,
    button_id: NodeId,
}

impl Counter {
    fn create(mut root: NodeMut) -> Self {
        let mut myself = Self::default();

        let root_id = root.id();
        let rdom = root.real_dom_mut();

        // create the counter
        let count = myself.count;
        let mut button = rdom.create_node(NodeType::Element(ElementNode {
            tag: "input".to_string(),
            attributes: [
                // supported types: button, checkbox, textbox, password, number, range
                ("type".to_string().into(), "range".to_string().into()),
                ("display".to_string().into(), "flex".to_string().into()),
                (("flex-direction", "style").into(), "row".to_string().into()),
                (
                    ("justify-content", "style").into(),
                    "center".to_string().into(),
                ),
                (("align-items", "style").into(), "center".to_string().into()),
                (
                    "value".to_string().into(),
                    format!("click me {count}").into(),
                ),
                (("width", "style").into(), "50%".to_string().into()),
                (("height", "style").into(), "10%".to_string().into()),
                ("min".to_string().into(), "20".to_string().into()),
                ("max".to_string().into(), "80".to_string().into()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        button.add_event_listener("input");
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
                format!("rgb({}, {}, {})", 255.0 - self.count * 2.0, 0, 0,),
            );
        };
    }

    fn handle_event(
        &mut self,
        _: &Arc<RwLock<RealDom>>,
        _: NodeId,
        event_type: &str,
        event: Rc<EventData>,
        _: bool,
    ) {
        if event_type == "input" {
            // when the button is clicked, increment the counter
            if let EventData::Form(input_event) = &*event {
                if let Ok(value) = input_event.value().parse::<f64>() {
                    self.count = value;
                }
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
        Counter::create(rdom.get_mut(root).unwrap())
    })
    .unwrap();
}
