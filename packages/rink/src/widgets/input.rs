use dioxus_native_core::{
    custom_element::CustomElement, node::OwnedAttributeDiscription, prelude::NodeType,
    real_dom::NodeImmutable,
};

use super::{
    checkbox::CheckBox, number::Number, password::Password, slider::Slider, textbox::TextBox,
    RinkWidget,
};
use crate::widgets::button::Button;

pub(crate) enum Input {
    Button(Button),
    CheckBox(CheckBox),
    TextBox(TextBox),
    Password(Password),
    Number(Number),
    Slider(Slider),
}

impl CustomElement for Input {
    const NAME: &'static str = "input";

    fn roots(&self) -> Vec<dioxus_native_core::NodeId> {
        match self {
            Input::Button(button) => button.roots(),
            Input::CheckBox(checkbox) => checkbox.roots(),
            Input::TextBox(textbox) => textbox.roots(),
            Input::Password(password) => password.roots(),
            Input::Number(number) => number.roots(),
            Input::Slider(slider) => slider.roots(),
        }
    }

    fn slot(&self) -> Option<dioxus_native_core::NodeId> {
        match self {
            Input::Button(button) => button.slot(),
            Input::CheckBox(checkbox) => checkbox.slot(),
            Input::TextBox(textbox) => textbox.slot(),
            Input::Password(password) => password.slot(),
            Input::Number(number) => number.slot(),
            Input::Slider(slider) => slider.slot(),
        }
    }

    fn create(mut root: dioxus_native_core::real_dom::NodeMut) -> Self {
        {
            // currently widgets are not allowed to have children
            let children = root.child_ids();
            let rdom = root.real_dom_mut();
            for child in children {
                if let Some(mut child) = rdom.get_mut(child) {
                    child.remove();
                }
            }
        }

        let node_type = root.node_type();
        let NodeType::Element(el) = &*node_type else {
            panic!("input must be an element")
        };
        let input_type = el
            .attributes
            .get(&OwnedAttributeDiscription {
                name: "type".to_string(),
                namespace: None,
            })
            .and_then(|value| value.as_text());
        match input_type
            .map(|type_| type_.trim().to_lowercase())
            .as_deref()
        {
            Some("button") => {
                drop(node_type);
                Input::Button(Button::create(root))
            }
            Some("checkbox") => {
                drop(node_type);
                Input::CheckBox(CheckBox::create(root))
            }
            Some("textbox") => {
                drop(node_type);
                Input::TextBox(TextBox::create(root))
            }
            Some("password") => {
                drop(node_type);
                Input::Password(Password::create(root))
            }
            Some("number") => {
                drop(node_type);
                Input::Number(Number::create(root))
            }
            Some("range") => {
                drop(node_type);
                Input::Slider(Slider::create(root))
            }
            _ => {
                drop(node_type);
                Input::TextBox(TextBox::create(root))
            }
        }
    }

    fn attributes_changed(
        &mut self,
        root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        match self {
            Input::Button(button) => {
                button.attributes_changed(root, attributes);
            }
            Input::CheckBox(checkbox) => {
                checkbox.attributes_changed(root, attributes);
            }
            Input::TextBox(textbox) => {
                textbox.attributes_changed(root, attributes);
            }
            Input::Password(password) => {
                password.attributes_changed(root, attributes);
            }
            Input::Number(number) => {
                number.attributes_changed(root, attributes);
            }
            Input::Slider(slider) => {
                slider.attributes_changed(root, attributes);
            }
        }
    }
}

impl RinkWidget for Input {
    fn handle_event(&mut self, event: &crate::Event, node: dioxus_native_core::real_dom::NodeMut) {
        match self {
            Input::Button(button) => {
                button.handle_event(event, node);
            }
            Input::CheckBox(checkbox) => {
                checkbox.handle_event(event, node);
            }
            Input::TextBox(textbox) => {
                textbox.handle_event(event, node);
            }
            Input::Password(password) => {
                password.handle_event(event, node);
            }
            Input::Number(number) => {
                number.handle_event(event, node);
            }
            Input::Slider(slider) => {
                slider.handle_event(event, node);
            }
        }
    }
}
