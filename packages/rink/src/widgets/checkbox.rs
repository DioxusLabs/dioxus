use std::collections::HashMap;

use dioxus_html::{input_data::keyboard_types::Key, HasKeyboardData};
use dioxus_native_core::{
    custom_element::CustomElement,
    node::OwnedAttributeDiscription,
    node_ref::AttributeMask,
    prelude::NodeType,
    real_dom::{ElementNodeMut, NodeImmutable, NodeMut, NodeTypeMut},
    NodeId,
};
use shipyard::UniqueView;

use crate::hooks::FormData;

use super::{RinkWidget, WidgetContext};

#[derive(Debug, Default)]
pub(crate) struct CheckBox {
    div_id: NodeId,
    text_id: NodeId,
    value: String,
    checked: bool,
}

impl CheckBox {
    fn width(el: &ElementNodeMut) -> String {
        if let Some(value) = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "width".to_string(),
                namespace: None,
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
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
        {
            value
        } else {
            "1px".to_string()
        }
    }

    fn update_size_attr(&mut self, el: &mut ElementNodeMut) {
        let width = Self::width(el);
        let height = Self::height(el);
        let single_char = width == "1px" || height == "1px";
        let border_style = if single_char { "none" } else { "solid" };
        el.set_attribute(
            OwnedAttributeDiscription {
                name: "border-style".to_string(),
                namespace: Some("style".to_string()),
            },
            border_style.to_string(),
        );
    }

    fn update_value_attr(&mut self, el: &ElementNodeMut) {
        self.value = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "value".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "on".to_string());
    }

    fn update_checked_attr(&mut self, el: &ElementNodeMut) {
        self.checked = el
            .get_attribute(&OwnedAttributeDiscription {
                name: "checked".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "false".to_string())
            == "true";
    }

    fn write_value(&self, mut root: NodeMut) {
        let single_char = {
            let node_type = root.node_type_mut();
            let NodeTypeMut::Element(el) = node_type else {
                panic!("input must be an element")
            };
            Self::width(&el) == "1px" || Self::height(&el) == "1px"
        };
        let rdom = root.real_dom_mut();

        if let Some(mut text) = rdom.get_mut(self.text_id) {
            let node_type = text.node_type_mut();
            let NodeTypeMut::Text(mut text) = node_type else {
                panic!("input must be an element")
            };
            let value = if single_char {
                if self.checked {
                    "☑"
                } else {
                    "☐"
                }
            } else if self.checked {
                "✓"
            } else {
                " "
            };
            *text.text_mut() = value.to_string();
        }
    }

    fn switch(&mut self, mut node: NodeMut) {
        let new_state = !self.checked;

        let data = FormData {
            value: new_state
                .then(|| self.value.to_string())
                .unwrap_or_default(),
            values: HashMap::new(),
            files: None,
        };
        {
            let ctx: UniqueView<WidgetContext> = node
                .real_dom_mut()
                .raw_world_mut()
                .borrow()
                .expect("expected widget context");
            ctx.send(crate::Event {
                id: self.div_id,
                name: "input",
                data: crate::EventData::Form(data),
                bubbles: true,
            });
        }

        self.checked = new_state;

        self.write_value(node);
    }
}

impl CustomElement for CheckBox {
    const NAME: &'static str = "input";

    fn roots(&self) -> Vec<NodeId> {
        vec![self.text_id]
    }

    fn create(mut root: dioxus_native_core::real_dom::NodeMut) -> Self {
        let node_type = root.node_type();
        let NodeType::Element(el) = &*node_type else {
            panic!("input must be an element")
        };

        let value = el
            .attributes
            .get(&OwnedAttributeDiscription {
                name: "value".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text())
            .map(|value| value.to_string());

        drop(node_type);

        let rdom = root.real_dom_mut();
        let text = rdom.create_node(String::new());
        let text_id = text.id();

        root.add_event_listener("click");
        root.add_event_listener("keydown");
        let div_id = root.id();

        let myself = Self {
            div_id,
            text_id,
            value: value.unwrap_or_default(),
            checked: false,
        };
        myself.write_value(root);

        myself
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
                    self.update_checked_attr(&el);
                }
                self.write_value(root);
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
                    if attrs.contains("value") {
                        self.update_value_attr(&el);
                    }
                    if attrs.contains("checked") {
                        self.update_checked_attr(&el);
                    }
                }
                if attrs.contains("checked") {
                    self.write_value(root);
                }
            }
        }
    }
}

impl RinkWidget for CheckBox {
    fn handle_event(&mut self, event: &crate::Event, node: dioxus_native_core::real_dom::NodeMut) {
        match event.name {
            "click" => self.switch(node),
            "keydown" => {
                if let crate::EventData::Keyboard(data) = &event.data {
                    if !data.is_auto_repeating()
                        && match data.key() {
                            Key::Character(c) if c == " " => true,
                            Key::Enter => true,
                            _ => false,
                        }
                    {
                        self.switch(node);
                    }
                }
            }
            _ => {}
        }
    }
}
