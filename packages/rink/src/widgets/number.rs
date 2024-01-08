use dioxus_html::{input_data::keyboard_types::Key, HasKeyboardData};
use dioxus_native_core::{
    custom_element::CustomElement,
    real_dom::{NodeImmutable, RealDom},
    NodeId,
};

use crate::EventData;

use super::{text_like::TextLike, RinkWidget};

#[derive(Debug, Default)]
pub(crate) struct Number {
    text: TextLike,
}

impl Number {
    fn increase(&mut self, rdom: &mut RealDom, id: NodeId) {
        let num = self.text.text().parse::<f64>().unwrap_or(0.0);
        self.text.set_text((num + 1.0).to_string(), rdom, id);
    }

    fn decrease(&mut self, rdom: &mut RealDom, id: NodeId) {
        let num = self.text.text().parse::<f64>().unwrap_or(0.0);
        self.text.set_text((num - 1.0).to_string(), rdom, id);
    }
}

impl CustomElement for Number {
    const NAME: &'static str = "input";

    fn roots(&self) -> Vec<NodeId> {
        self.text.roots()
    }

    fn create(mut root: dioxus_native_core::real_dom::NodeMut) -> Self {
        Number {
            text: TextLike::create(root.reborrow()),
        }
    }

    fn attributes_changed(
        &mut self,
        root: dioxus_native_core::real_dom::NodeMut,
        attributes: &dioxus_native_core::node_ref::AttributeMask,
    ) {
        self.text.attributes_changed(root, attributes)
    }
}

impl RinkWidget for Number {
    fn handle_event(
        &mut self,
        event: &crate::Event,
        mut node: dioxus_native_core::real_dom::NodeMut,
    ) {
        if event.name == "keydown" {
            if let EventData::Keyboard(data) = &event.data {
                let key = data.key();
                let is_num_like = match key.clone() {
                    Key::ArrowLeft | Key::ArrowRight | Key::Backspace => true,
                    Key::Character(c)
                        if c == "." || c == "-" || c.chars().all(|c| c.is_numeric()) =>
                    {
                        true
                    }
                    _ => false,
                };

                if is_num_like {
                    self.text.handle_event(event, node)
                } else {
                    let id = node.id();
                    let rdom = node.real_dom_mut();
                    match key {
                        Key::ArrowUp => {
                            self.increase(rdom, id);
                        }
                        Key::ArrowDown => {
                            self.decrease(rdom, id);
                        }
                        _ => (),
                    }
                }
                return;
            }
        }

        self.text.handle_event(event, node)
    }
}
