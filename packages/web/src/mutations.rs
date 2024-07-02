use crate::dom::WebsysDom;
use dioxus_core::prelude::*;
use dioxus_core::WriteMutations;
use dioxus_core::{AttributeValue, ElementId};
use dioxus_html::event_bubbles;
use dioxus_interpreter_js::minimal_bindings;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

impl WebsysDom {
    pub(crate) fn create_template_node(&self, v: &TemplateNode) -> web_sys::Node {
        use TemplateNode::*;
        match v {
            Element {
                tag,
                namespace,
                attrs,
                children,
                ..
            } => {
                let el = match namespace {
                    Some(ns) => self.document.create_element_ns(Some(ns), tag).unwrap(),
                    None => self.document.create_element(tag).unwrap(),
                };
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } = attr
                    {
                        minimal_bindings::setAttributeInner(
                            el.clone().into(),
                            name,
                            JsValue::from_str(value),
                            *namespace,
                        );
                    }
                }
                for child in *children {
                    let _ = el.append_child(&self.create_template_node(child));
                }
                el.dyn_into().unwrap()
            }
            Text { text } => self.document.create_text_node(text).dyn_into().unwrap(),
            Dynamic { .. } => {
                let placeholder = self.document.create_comment("placeholder");
                placeholder.dyn_into().unwrap()
            }
        }
    }

    pub fn flush_edits(&mut self) {
        self.interpreter.flush();

        // Now that we've flushed the edits and the dom nodes exist, we can send the mounted events.
        #[cfg(feature = "mounted")]
        self.flush_queued_mounted_events();
    }

    #[cfg(feature = "mounted")]
    fn flush_queued_mounted_events(&mut self) {
        for id in self.queued_mounted_events.drain(..) {
            let node = self.interpreter.base().get_node(id.0 as u32);
            if let Some(element) = node.dyn_ref::<web_sys::Element>() {
                let _ = self.event_channel.unbounded_send(crate::dom::UiEvent {
                    name: "mounted".to_string(),
                    bubbles: false,
                    element: id,
                    data: dioxus_html::PlatformEventData::new(Box::new(element.clone())),
                });
            }
        }
    }

    #[cfg(feature = "mounted")]
    pub(crate) fn send_mount_event(&mut self, id: ElementId) {
        self.queued_mounted_events.push(id);
    }

    #[inline]
    fn only_write_templates(&self) -> bool {
        #[cfg(feature = "hydrate")]
        {
            self.only_write_templates
        }
        #[cfg(not(feature = "hydrate"))]
        {
            false
        }
    }
}

impl WriteMutations for WebsysDom {
    fn register_template(&mut self, template: Template) {
        let mut roots = vec![];
        for root in template.roots {
            roots.push(self.create_template_node(root))
        }
        self.templates
            .insert(template.name.to_owned(), self.max_template_id);
        self.interpreter
            .base()
            .save_template(roots, self.max_template_id);
        self.max_template_id += 1
    }

    fn append_children(&mut self, id: ElementId, m: usize) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.append_children(id.0 as u32, m as u16)
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter
            .assign_id(path.as_ptr() as u32, path.len() as u8, id.0 as u32)
    }

    fn create_placeholder(&mut self, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.create_placeholder(id.0 as u32)
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.create_text_node(value, id.0 as u32)
    }

    fn hydrate_text_node(&mut self, path: &'static [u8], value: &str, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter
            .hydrate_text(path.as_ptr() as u32, path.len() as u8, value, id.0 as u32)
    }

    fn load_template(&mut self, name: &'static str, index: usize, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        if let Some(tmpl_id) = self.templates.get(name) {
            self.interpreter
                .load_template(*tmpl_id, index as u16, id.0 as u32)
        }
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.replace_with(id.0 as u32, m as u16)
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter
            .replace_placeholder(path.as_ptr() as u32, path.len() as u8, m as u16)
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.insert_after(id.0 as u32, m as u16)
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.insert_before(id.0 as u32, m as u16)
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        if self.only_write_templates() {
            return;
        }
        match value {
            AttributeValue::Text(txt) => {
                self.interpreter
                    .set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
            }
            AttributeValue::Float(f) => self.interpreter.set_attribute(
                id.0 as u32,
                name,
                &f.to_string(),
                ns.unwrap_or_default(),
            ),
            AttributeValue::Int(n) => self.interpreter.set_attribute(
                id.0 as u32,
                name,
                &n.to_string(),
                ns.unwrap_or_default(),
            ),
            AttributeValue::Bool(b) => self.interpreter.set_attribute(
                id.0 as u32,
                name,
                if *b { "true" } else { "false" },
                ns.unwrap_or_default(),
            ),
            AttributeValue::None => {
                self.interpreter
                    .remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
            }
            _ => unreachable!(),
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.set_text(id.0 as u32, value)
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        // mounted events are fired immediately after the element is mounted.
        if name == "mounted" {
            #[cfg(feature = "mounted")]
            self.send_mount_event(id);
            return;
        }

        self.interpreter
            .new_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        if name == "mounted" {
            return;
        }

        self.interpreter
            .remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_node(&mut self, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.remove(id.0 as u32)
    }

    fn push_root(&mut self, id: ElementId) {
        if self.only_write_templates() {
            return;
        }
        self.interpreter.push_root(id.0 as u32)
    }
}
