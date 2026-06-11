use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use dioxus_core_types::event_bubbles;
use dioxus_interpreter_js::minimal_bindings;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

impl WebsysDom {
    /// Build the cloneable DOM tree for one root of a template, *omitting*
    /// `TemplateNode::Dynamic` positions entirely. Returns `None` for a
    /// Dynamic slot — the caller must NOT append it as a child; instead the
    /// slot's position is recorded out-of-band in the slot-anchor table sent
    /// to JS at `save_template` time, and the JS-side `loadChild` learns to
    /// reinterpret byte paths that count the slot's logical position.
    pub(crate) fn create_template_node(&self, v: &TemplateNode) -> Option<web_sys::Node> {
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
                    if let Some(child_node) = self.create_template_node(child) {
                        let _ = el.append_child(&child_node);
                    }
                }
                Some(el.dyn_into().unwrap())
            }
            Text { text } => Some(self.document.create_text_node(text).dyn_into().unwrap()),
            Dynamic { .. } => None,
        }
    }

    pub fn flush_edits(&mut self) {
        self.interpreter.flush();

        // Now that we've flushed the edits and the dom nodes exist, we can send the mounted events.
        #[cfg(feature = "mounted")]
        self.flush_queued_mounted_events();
    }

    #[cfg(feature = "mounted")]
    pub(crate) fn flush_queued_mounted_events(&mut self) {
        for id in self.queued_mounted_events.drain(..) {
            let node = self.interpreter.base().get_node(id.0 as u32);
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
            // Build one DOM root per `template.roots()` entry. A root that
            // is itself a `Dynamic { .. }` has no DOM representation; we
            // emit `JsValue::NULL` in its place so the JS `templates[id]`
            // array stays aligned with the root index. The JS side never
            // clones a null entry — root-level Dynamic slots are routed
            // through `create_dynamic_node` directly by the core diff.
            let roots = js_sys::Array::new();
            for root in template.roots() {
                match self.create_template_node(root) {
                    Some(node) => {
                        roots.push(node.as_ref());
                    }
                    None => {
                        // Root-level `TemplateNode::Dynamic`: never cloned;
                        // `create_dynamic_node` runs directly from the core
                        // diff. Send `null` so JS keeps root indices dense
                        // without allocating a phantom DOM node.
                        roots.push(&JsValue::NULL);
                    }
                }
            }
            let id = self.templates.len() as u16;
            self.templates.insert(template, id);

            // Build the slot-path table. The `node_paths` from the template
            // describe every Dynamic slot as `[root_idx, child0, child1, ...]`.
            // Group them by root index so JS can attach per-root slot maps to
            // each `templates[id][root_idx]` entry.
            let slot_paths = js_sys::Array::new();
            for path in template.node_paths() {
                // Each path becomes a Uint8Array. Include the root_idx prefix
                // so JS can dispatch into the right root's slot map.
                let arr = js_sys::Uint8Array::new_with_length(path.len() as u32);
                arr.copy_from(path);
                slot_paths.push(&arr);
            }

            self.interpreter.base().save_template(roots, id, slot_paths);
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

    fn insert_children_at_path(&mut self, id: ElementId, path: &'static [u8], m: usize) {
        if self.skip_mutations() {
            return;
        }
        self.interpreter.insert_children_at_path(
            id.0 as u32,
            path.as_ptr() as u32,
            path.len() as u8,
            m as u16,
        )
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
        if self.skip_mutations() {
            return;
        }
        if name == "mounted" {
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
