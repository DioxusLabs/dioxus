use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use dioxus_core_types::event_bubbles;
use dioxus_interpreter_js::minimal_bindings;
use dioxus_web_sys_events::QueueMountedEvents;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Flush pending edits to the dom, then dispatch the mounted events queued while rendering
/// now that the dom nodes exist.
pub(crate) fn flush_edits(dom: &mut QueueMountedEvents<WebsysDom>) {
    let mounted_events = dom.take_mounted_events();
    dom.interpreter.flush();
    dom.dispatch_mounted_events(mounted_events);
}

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

    /// Dispatch mounted events against the live elements. This should only be called after the
    /// edits that created the elements have been flushed to the dom.
    pub(crate) fn dispatch_mounted_events(&mut self, ids: impl IntoIterator<Item = ElementId>) {
        #[cfg(feature = "mounted")]
        for id in ids {
            let node = self.interpreter.base().get_node(id.0 as u32);
            if let Some(element) = node.dyn_ref::<web_sys::Element>() {
                dioxus_web_sys_events::dispatch_mounted_event(&self.runtime, id, element.clone());
            }
        }
        #[cfg(not(feature = "mounted"))]
        let _ = ids;
    }

    #[inline]
    fn skip_mutations(&self) -> bool {
        #[cfg(feature = "hydrate")]
        {
            self.skip_mutations
        }
        #[cfg(not(feature = "hydrate"))]
        {
            false
        }
    }
}

impl WriteMutations for WebsysDom {
    fn append_children(&mut self, id: ElementId, m: usize) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.append_children(id.0 as u32, m as u16)
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter
            .assign_id(path.as_ptr() as u32, path.len() as u8, id.0 as u32)
    }

    fn create_placeholder(&mut self, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.create_placeholder(id.0 as u32)
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.create_text_node(value, id.0 as u32)
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        let tmpl_id = self.templates.get(&template).cloned().unwrap_or_else(|| {
            let mut roots = vec![];
            for root in template.roots() {
                roots.push(self.create_template_node(root))
            }
            let id = self.templates.len() as u16;
            self.templates.insert(template, id);
            self.interpreter.base().save_template(roots, id);
            id
        });

        self.interpreter
            .load_template(tmpl_id, index as u16, id.0 as u32)
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.replace_with(id.0 as u32, m as u16)
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter
            .replace_placeholder(path.as_ptr() as u32, path.len() as u8, m as u16)
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.insert_after(id.0 as u32, m as u16)
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        if self.skip_mutations() {
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
        if self.skip_mutations() {
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
        if self.skip_mutations() {
            return;
        }
        self.interpreter.set_text(id.0 as u32, value)
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter
            .new_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter
            .remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_node(&mut self, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.remove(id.0 as u32)
    }

    fn push_root(&mut self, id: ElementId) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.push_root(id.0 as u32)
    }
}
