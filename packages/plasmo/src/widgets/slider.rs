use std::collections::HashMap;

use dioxus_html::{
    input_data::keyboard_types::Key, prelude::*, HasKeyboardData, SerializedKeyboardData,
    SerializedMouseData,
};
use dioxus_native_core::{
    custom_element::CustomElement,
    node::{OwnedAttributeDiscription, OwnedAttributeValue},
    node_ref::AttributeMask,
    prelude::{ElementNode, NodeType},
    real_dom::{ElementNodeMut, NodeImmutable, NodeMut, NodeTypeMut, RealDom},
    NodeId,
};
use shipyard::UniqueView;

use super::{RinkWidget, WidgetContext};
use crate::hooks::FormData;
use crate::{query::get_layout, Event, EventData, Query};

#[derive(Debug)]
pub(crate) struct Slider {
    div_wrapper: NodeId,
    pre_cursor_div: NodeId,
    post_cursor_div: NodeId,
    min: f64,
    max: f64,
    step: Option<f64>,
    value: f64,
    border: bool,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            div_wrapper: Default::default(),
            pre_cursor_div: Default::default(),
            post_cursor_div: Default::default(),
            min: 0.0,
            max: 100.0,
            step: None,
            value: 0.0,
            border: false,
        }
    }
}

impl Slider {
    fn size(&self) -> f64 {
        self.max - self.min
    }

    fn step(&self) -> f64 {
        self.step.unwrap_or(self.size() / 10.0)
    }

    fn width(el: &ElementNodeMut) -> String {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "width".to_string(),
                namespace: Some("style".to_string()),
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            value
        } else {
            "1px".to_string()
        }
    }

    fn height(el: &ElementNodeMut) -> String {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "height".to_string(),
                namespace: Some("style".to_string()),
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            value
        } else {
            "1px".to_string()
        }
    }

    fn update_min_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "min".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            self.min = value.parse().ok().unwrap_or(0.0);
        }
    }

    fn update_max_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "max".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            self.max = value.parse().ok().unwrap_or(100.0);
        }
    }

    fn update_step_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "step".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            self.step = value.parse().ok();
        }
    }

    fn update_size_attr(&mut self, el: &mut ElementNodeMut) {
        let width = Self::width(el);
        let height = Self::height(el);
        let single_char = width
            .strip_prefix("px")
            .and_then(|n| n.parse::<u32>().ok().filter(|num| *num > 3))
            .is_some()
            || height
                .strip_prefix("px")
                .and_then(|n| n.parse::<u32>().ok().filter(|num| *num > 3))
                .is_some();
        self.border = !single_char;
        let border_style = if self.border { "solid" } else { "none" };
        el.set_attribute(
            OwnedAttributeDiscription {
                name: "border-style".to_string(),
                namespace: Some("style".to_string()),
            },
            border_style.to_string(),
        );
    }

    fn update_value(&mut self, new: f64) {
        self.value = new.clamp(self.min, self.max);
    }

    fn update_value_attr(&mut self, el: &ElementNodeMut) {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "value".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            self.update_value(value.parse().ok().unwrap_or(0.0));
        }
    }

    fn write_value(&self, rdom: &mut RealDom, id: NodeId) {
        let value_percent = (self.value - self.min) / self.size() * 100.0;

        if let Some(mut div) = rdom.get_mut(self.pre_cursor_div) {
            let node_type = div.node_type_mut();
            let NodeTypeMut::Element(mut element) = node_type else {
                panic!("input must be an element")
            };
            element.set_attribute(
                OwnedAttributeDiscription {
                    name: "width".to_string(),
                    namespace: Some("style".to_string()),
                },
                format!("{}%", value_percent),
            );
        }

        if let Some(mut div) = rdom.get_mut(self.post_cursor_div) {
            let node_type = div.node_type_mut();
            let NodeTypeMut::Element(mut element) = node_type else {
                panic!("input must be an element")
            };
            element.set_attribute(
                OwnedAttributeDiscription {
                    name: "width".to_string(),
                    namespace: Some("style".to_string()),
                },
                format!("{}%", 100.0 - value_percent),
            );
        }

        // send the event
        let world = rdom.raw_world_mut();

        {
            let ctx: UniqueView<WidgetContext> = world.borrow().expect("expected widget context");

            let data = FormData {
                value: self.value.to_string(),
                values: HashMap::new(),
                files: None,
            };
            ctx.send(Event {
                id,
                name: "input",
                data: EventData::Form(data),
                bubbles: true,
            });
        }
    }

    fn handle_keydown(&mut self, mut root: NodeMut, data: &SerializedKeyboardData) {
        let key = data.key();

        let step = self.step();
        match key {
            Key::ArrowDown | Key::ArrowLeft => {
                self.update_value(self.value - step);
            }
            Key::ArrowUp | Key::ArrowRight => {
                self.update_value(self.value + step);
            }
            _ => {
                return;
            }
        }

        let id = root.id();

        let rdom = root.real_dom_mut();
        self.write_value(rdom, id);
    }

    fn handle_mousemove(&mut self, mut root: NodeMut, data: &SerializedMouseData) {
        if !data.held_buttons().is_empty() {
            let id = root.id();
            let rdom = root.real_dom_mut();
            let world = rdom.raw_world_mut();
            let taffy = {
                let query: UniqueView<Query> = world.borrow().unwrap();
                query.stretch.clone()
            };

            let taffy = taffy.lock().unwrap();

            let layout = get_layout(rdom.get(self.div_wrapper).unwrap(), &taffy).unwrap();

            let width = layout.size.width as f64;
            let offset = data.element_coordinates();
            self.update_value(self.min + self.size() * offset.x / width);

            self.write_value(rdom, id);
        }
    }
}

impl CustomElement for Slider {
    const NAME: &'static str = "input";

    fn roots(&self) -> Vec<NodeId> {
        vec![self.div_wrapper]
    }

    fn create(mut root: dioxus_native_core::real_dom::NodeMut) -> Self {
        let node_type = root.node_type();
        let NodeType::Element(el) = &*node_type else {
            panic!("input must be an element")
        };

        let value = el.attributes.get(&OwnedAttributeDiscription {
            name: "value".to_string(),
            namespace: None,
        });
        let value = value
            .and_then(|value| match value {
                OwnedAttributeValue::Text(text) => text.as_str().parse().ok(),
                OwnedAttributeValue::Float(float) => Some(*float),
                OwnedAttributeValue::Int(int) => Some(*int as f64),
                _ => None,
            })
            .unwrap_or(0.0);

        drop(node_type);

        let rdom = root.real_dom_mut();

        let pre_cursor_div = rdom.create_node(NodeType::Element(ElementNode {
            tag: "div".to_string(),
            attributes: [(
                OwnedAttributeDiscription {
                    name: "background-color".to_string(),
                    namespace: Some("style".to_string()),
                },
                "rgba(10,10,10,0.5)".to_string().into(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        let pre_cursor_div_id = pre_cursor_div.id();

        let cursor_text = rdom.create_node("|".to_string());
        let cursor_text_id = cursor_text.id();
        let mut cursor_span = rdom.create_node(NodeType::Element(ElementNode {
            tag: "div".to_string(),
            attributes: [].into_iter().collect(),
            ..Default::default()
        }));
        cursor_span.add_child(cursor_text_id);
        let cursor_span_id = cursor_span.id();

        let post_cursor_div = rdom.create_node(NodeType::Element(ElementNode {
            tag: "span".to_string(),
            attributes: [
                (
                    OwnedAttributeDiscription {
                        name: "width".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "100%".to_string().into(),
                ),
                (
                    OwnedAttributeDiscription {
                        name: "background-color".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "rgba(10,10,10,0.5)".to_string().into(),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        let post_cursor_div_id = post_cursor_div.id();

        let mut div_wrapper = rdom.create_node(NodeType::Element(ElementNode {
            tag: "div".to_string(),
            attributes: [
                (
                    OwnedAttributeDiscription {
                        name: "display".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "flex".to_string().into(),
                ),
                (
                    OwnedAttributeDiscription {
                        name: "flex-direction".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "row".to_string().into(),
                ),
                (
                    OwnedAttributeDiscription {
                        name: "width".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "100%".to_string().into(),
                ),
                (
                    OwnedAttributeDiscription {
                        name: "height".to_string(),
                        namespace: Some("style".to_string()),
                    },
                    "100%".to_string().into(),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        }));
        let div_wrapper_id = div_wrapper.id();
        div_wrapper.add_child(pre_cursor_div_id);
        div_wrapper.add_child(cursor_span_id);
        div_wrapper.add_child(post_cursor_div_id);

        root.add_event_listener("mousemove");
        root.add_event_listener("mousedown");
        root.add_event_listener("keydown");

        Self {
            pre_cursor_div: pre_cursor_div_id,
            post_cursor_div: post_cursor_div_id,
            div_wrapper: div_wrapper_id,
            value,
            ..Default::default()
        }
    }

    fn attributes_changed(
        &mut self,
        mut root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        match attributes {
            AttributeMask::All => {
                {
                    let node_type = root.node_type_mut();
                    let NodeTypeMut::Element(mut el) = node_type else {
                        panic!("input must be an element")
                    };
                    self.update_value_attr(&el);
                    self.update_size_attr(&mut el);
                    self.update_max_attr(&el);
                    self.update_min_attr(&el);
                    self.update_step_attr(&el);
                }
                let id = root.id();
                self.write_value(root.real_dom_mut(), id);
            }
            AttributeMask::Some(attrs) => {
                {
                    let node_type = root.node_type_mut();
                    let NodeTypeMut::Element(mut el) = node_type else {
                        panic!("input must be an element")
                    };
                    if attrs.contains("width") || attrs.contains("height") {
                        self.update_size_attr(&mut el);
                    }
                    if attrs.contains("max") {
                        self.update_max_attr(&el);
                    }
                    if attrs.contains("min") {
                        self.update_min_attr(&el);
                    }
                    if attrs.contains("step") {
                        self.update_step_attr(&el);
                    }
                    if attrs.contains("value") {
                        self.update_value_attr(&el);
                    }
                }
                if attrs.contains("value") {
                    let id = root.id();
                    self.write_value(root.real_dom_mut(), id);
                }
            }
        }
    }
}

impl RinkWidget for Slider {
    fn handle_event(&mut self, event: &crate::Event, node: dioxus_native_core::real_dom::NodeMut) {
        match event.name {
            "keydown" => {
                if let EventData::Keyboard(data) = &event.data {
                    self.handle_keydown(node, data);
                }
            }

            "mousemove" => {
                if let EventData::Mouse(data) = &event.data {
                    self.handle_mousemove(node, data);
                }
            }

            "mousedown" => {
                if let EventData::Mouse(data) = &event.data {
                    self.handle_mousemove(node, data);
                }
            }

            _ => {}
        }
    }
}
