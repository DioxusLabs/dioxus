use dioxus_html::event_bubbles;

use dioxus_core::{TemplateAttribute, TemplateNode, WriteMutations};
use sledgehammer_utils::rustc_hash::FxHashMap;

use crate::binary_protocol::Channel;

/// The state needed to apply mutations to a channel. This state should be kept across all mutations for the app
#[derive(Default)]
pub struct MutationState {
    /// The maximum number of templates that we have registered
    max_template_count: u16,
    /// The currently registered templates with the template ids
    templates: FxHashMap<String, u16>,
    /// The channel that we are applying mutations to
    channel: Channel,
}

impl MutationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn export_memory(&mut self) -> Vec<u8> {
        let bytes: Vec<_> = self.channel.export_memory().collect();
        self.channel.reset();
        bytes
    }

    pub fn write_memory_into(&mut self, buffer: &mut Vec<u8>) {
        buffer.extend(self.channel.export_memory());
        self.channel.reset();
    }

    pub fn channel(&mut self) -> &mut Channel {
        &mut self.channel
    }

    pub fn channel_mut(&mut self) -> &mut Channel {
        &mut self.channel
    }

    fn create_template_node(&mut self, node: &'static TemplateNode) {
        use TemplateNode::*;
        match node {
            Element {
                tag,
                namespace,
                attrs,
                children,
                ..
            } => {
                // Push the current node onto the stack
                match namespace {
                    Some(ns) => self.channel.create_element_ns(tag, ns),
                    None => self.channel.create_element(tag),
                }
                // Set attributes on the current node
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } = attr
                    {
                        self.channel
                            .set_top_attribute(name, value, namespace.unwrap_or_default())
                    }
                }
                // Add each child to the stack
                for child in *children {
                    self.create_template_node(child);
                }
                // Add all children to the parent
                self.channel.append_children_to_top(children.len() as u16);
            }
            Text { text } => self.channel.create_raw_text(text),
            DynamicText { .. } => self.channel.create_raw_text("p"),
            Dynamic { .. } => self.channel.add_placeholder(),
        }
    }
}

impl WriteMutations for MutationState {
    fn register_template(&mut self, template: dioxus_core::prelude::Template) {
        let current_max_template_count = self.max_template_count;
        for root in template.roots.iter() {
            self.create_template_node(root);
            self.templates
                .insert(template.name.to_owned(), current_max_template_count);
        }
        self.channel
            .add_templates(current_max_template_count, template.roots.len() as u16);

        self.max_template_count += 1;
    }

    fn append_children(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.channel.append_children(id.0 as u32, m as u16);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: dioxus_core::ElementId) {
        self.channel.assign_id(path, id.0 as u32);
    }

    fn create_placeholder(&mut self, id: dioxus_core::ElementId) {
        self.channel.create_placeholder(id.0 as u32);
    }

    fn create_text_node(&mut self, value: &str, id: dioxus_core::ElementId) {
        self.channel.create_text_node(value, id.0 as u32);
    }

    fn hydrate_text_node(&mut self, path: &'static [u8], value: &str, id: dioxus_core::ElementId) {
        self.channel.hydrate_text(path, value, id.0 as u32);
    }

    fn load_template(&mut self, name: &'static str, index: usize, id: dioxus_core::ElementId) {
        if let Some(tmpl_id) = self.templates.get(name) {
            self.channel
                .load_template(*tmpl_id, index as u16, id.0 as u32)
        }
    }

    fn replace_node_with(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.channel.replace_with(id.0 as u32, m as u16);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.channel.replace_placeholder(path, m as u16);
    }

    fn insert_nodes_after(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.channel.insert_after(id.0 as u32, m as u16);
    }

    fn insert_nodes_before(&mut self, id: dioxus_core::ElementId, m: usize) {
        self.channel.insert_before(id.0 as u32, m as u16);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &dioxus_core::AttributeValue,
        id: dioxus_core::ElementId,
    ) {
        match value {
            dioxus_core::AttributeValue::Text(txt) => {
                self.channel
                    .set_attribute(id.0 as u32, name, txt, ns.unwrap_or_default())
            }
            dioxus_core::AttributeValue::Float(f) => self.channel.set_attribute(
                id.0 as u32,
                name,
                &f.to_string(),
                ns.unwrap_or_default(),
            ),
            dioxus_core::AttributeValue::Int(n) => self.channel.set_attribute(
                id.0 as u32,
                name,
                &n.to_string(),
                ns.unwrap_or_default(),
            ),
            dioxus_core::AttributeValue::Bool(b) => self.channel.set_attribute(
                id.0 as u32,
                name,
                if *b { "true" } else { "false" },
                ns.unwrap_or_default(),
            ),
            dioxus_core::AttributeValue::None => {
                self.channel
                    .remove_attribute(id.0 as u32, name, ns.unwrap_or_default())
            }
            _ => unreachable!("Any attributes are not supported by the current renderer"),
        }
    }

    fn set_node_text(&mut self, value: &str, id: dioxus_core::ElementId) {
        self.channel.set_text(id.0 as u32, value);
    }

    fn create_event_listener(&mut self, name: &'static str, id: dioxus_core::ElementId) {
        self.channel
            .new_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: dioxus_core::ElementId) {
        self.channel
            .remove_event_listener(name, id.0 as u32, event_bubbles(name) as u8);
    }

    fn remove_node(&mut self, id: dioxus_core::ElementId) {
        self.channel.remove(id.0 as u32);
    }

    fn push_root(&mut self, _id: dioxus_core::ElementId) {
        self.channel.push_root(0);
    }
}
