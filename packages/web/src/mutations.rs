use crate::dom::WebsysDom;
use dioxus_core::{AttributeValue, ElementId, WriteMutations};
use dioxus_core_types::event_bubbles;
#[cfg(feature = "mounted")]
use wasm_bindgen::JsCast;

impl WebsysDom {
    pub fn flush_edits(&mut self) {
        self.interpreter.flush();

        // Now that we've flushed the edits and the dom nodes exist, we can send the mounted events.
        #[cfg(feature = "mounted")]
        self.flush_queued_mounted_events();
    }

    #[cfg(feature = "mounted")]
    pub(crate) fn flush_queued_mounted_events(&mut self) {
        for id in self.queued_mounted_events.drain(..) {
            let node = self.interpreter.base().get_node(id.raw() as u32);
            if let Some(element) = node.dyn_ref::<web_sys::Element>() {
                let event = dioxus_core::Event::new(
                    std::rc::Rc::new(dioxus_html::PlatformEventData::new(Box::new(
                        element.clone(),
                    ))) as std::rc::Rc<dyn std::any::Any>,
                    false,
                );
                let name = "mounted";
                self.runtime.handle_event(name, event, id)
            }
        }
    }

    #[cfg(feature = "mounted")]
    pub(crate) fn send_mount_event(&mut self, id: ElementId) {
        self.queued_mounted_events.push(id);
    }
}

impl WriteMutations for WebsysDom {
    fn push_id(&mut self, id: ElementId) {
        #[cfg(feature = "mounted")]
        {
            self.current_writing_id = id;
        }
        self.interpreter.push_id(id.raw() as u32)
    }

    fn pop_id(&mut self, id: ElementId) {
        self.interpreter.pop_id(id.raw() as u32)
    }

    fn child(&mut self, index: usize) {
        self.interpreter.child(index as u32)
    }

    fn pop(&mut self) {
        self.interpreter.pop()
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        self.interpreter
            .create_element_top(tag, ns.unwrap_or_default())
    }

    fn create_text(&mut self, value: &str) {
        self.interpreter.create_text(value)
    }

    fn clone(&mut self) {
        self.interpreter.clone_node()
    }

    fn append_children(&mut self, m: usize) {
        self.interpreter.append_children_top(m as u16)
    }

    fn replace_with(&mut self, m: usize) {
        self.interpreter.replace_top_with(m as u16)
    }

    fn insert_after(&mut self, m: usize) {
        self.interpreter.insert_after_top(m as u16)
    }

    fn insert_before(&mut self, m: usize) {
        self.interpreter.insert_before_top(m as u16)
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        let text_value = match value {
            AttributeValue::Text(txt) => Some(txt),
            AttributeValue::Float(f) => Some(&f.to_string()),
            AttributeValue::Int(n) => Some(&n.to_string()),
            AttributeValue::Bool(b) => Some(if *b { "true" } else { "false" }),
            AttributeValue::None => None,
            _ => unreachable!(),
        };
        if let Some(text_value) = text_value {
            self.interpreter
                .set_current_attribute(name, &text_value, ns.unwrap_or_default())
        } else {
            self.interpreter
                .remove_current_attribute(name, ns.unwrap_or_default())
        }
    }

    fn set_text(&mut self, value: &str) {
        self.interpreter.set_top_text(value)
    }

    fn add_event_listener(&mut self, name: &str) {
        // mounted events are fired immediately after the element is mounted.
        if name == "mounted" {
            #[cfg(feature = "mounted")]
            self.send_mount_event(self.current_writing_id);
            return;
        }

        self.interpreter
            .new_top_event_listener(name, event_bubbles(name) as u8);
    }

    fn remove_event_listener(&mut self, name: &str) {
        if name == "mounted" {
            return;
        }

        self.interpreter
            .remove_top_event_listener(name, event_bubbles(name) as u8);
    }

    fn remove(&mut self) {
        self.interpreter.remove_top()
    }
}
