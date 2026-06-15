use crate::unified_bindings::Interpreter as Channel;
use dioxus_core::WriteMutations;
use dioxus_core_types::event_bubbles;

/// The state needed to apply mutations to a channel. This state should be kept across all mutations for the app
#[derive(Default)]
pub struct MutationState {
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
}

impl WriteMutations for MutationState {
    fn push_id(&mut self, id: dioxus_core::ElementId) {
        self.channel.push_id(id.raw() as u32);
    }

    fn pop_id(&mut self, id: dioxus_core::ElementId) {
        self.channel.pop_id(id.raw() as u32);
    }

    fn child(&mut self, index: usize) {
        self.channel.child(index as u32);
    }

    fn pop(&mut self) {
        self.channel.pop();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        self.channel.create_element_top(tag, ns.unwrap_or_default());
    }

    fn create_text(&mut self, value: &str) {
        self.channel.create_text(value);
    }

    fn clone(&mut self) {
        self.channel.clone_node();
    }

    fn append_children(&mut self, m: usize) {
        self.channel.append_children_top(m as u16);
    }

    fn replace_with(&mut self, m: usize) {
        self.channel.replace_top_with(m as u16);
    }

    fn insert_after(&mut self, m: usize) {
        self.channel.insert_after_top(m as u16);
    }

    fn insert_before(&mut self, m: usize) {
        self.channel.insert_before_top(m as u16);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &dioxus_core::AttributeValue) {
        match value {
            dioxus_core::AttributeValue::Text(txt) => {
                self.channel
                    .set_current_attribute(name, txt, ns.unwrap_or_default())
            }
            dioxus_core::AttributeValue::Float(f) => {
                self.channel
                    .set_current_attribute(name, &f.to_string(), ns.unwrap_or_default())
            }
            dioxus_core::AttributeValue::Int(n) => {
                self.channel
                    .set_current_attribute(name, &n.to_string(), ns.unwrap_or_default())
            }
            dioxus_core::AttributeValue::Bool(b) => self.channel.set_current_attribute(
                name,
                if *b { "true" } else { "false" },
                ns.unwrap_or_default(),
            ),
            dioxus_core::AttributeValue::None => self
                .channel
                .remove_current_attribute(name, ns.unwrap_or_default()),
            _ => unreachable!("Any attributes are not supported by the current renderer"),
        }
    }

    fn set_text(&mut self, value: &str) {
        self.channel.set_top_text(value);
    }

    fn add_event_listener(&mut self, name: &str) {
        // note that we use the foreign event listener here instead of the native one
        // the native method assumes we have direct access to the dom, which we don't.
        self.channel
            .foreign_top_event_listener(name, event_bubbles(name) as u8);
    }

    fn remove_event_listener(&mut self, name: &str) {
        self.channel
            .remove_top_event_listener(name, event_bubbles(name) as u8);
    }

    fn remove(&mut self) {
        self.channel.remove_top();
    }
}
