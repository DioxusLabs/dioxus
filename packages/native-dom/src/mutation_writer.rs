//! Integration between Dioxus and Blitz
use crate::{NodeId, qual_name, trace, write_once_attr::WriteOnceAttr};
use blitz_dom::{BaseDocument, Document as _, DocumentMutator, PlainDocument, Widget};
use blitz_traits::events::DomEventKind;
use dioxus_core::{AttributeValue, ElementId, WriteMutations};
use std::str::FromStr as _;

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Stack machine state for applying dioxus mutations
    pub(crate) stack: Vec<(NodeId, Option<ElementId>)>,
    /// Mapping from vdom ElementId -> rdom NodeId
    pub(crate) node_id_mapping: Vec<Option<NodeId>>,
    /// Count of each handler type
    pub(crate) event_handler_counts: [u32; 32],
    /// Mounted events queued as elements are mounted
    pub(crate) queued_mounted_events: Vec<ElementId>,
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create(root_id: usize) -> Self {
        Self {
            stack: vec![(root_id, Some(ElementId::ROOT))],
            node_id_mapping: vec![Some(root_id)],
            event_handler_counts: [0; 32],
            queued_mounted_events: Vec::new(),
        }
    }

    /// Convert an ElementId to a NodeId
    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.try_element_to_node_id(element_id).unwrap()
    }

    /// Attempt to convert an ElementId to a NodeId. This will return None if the ElementId is not in the RealDom.
    pub fn try_element_to_node_id(&self, element_id: ElementId) -> Option<NodeId> {
        self.node_id_mapping
            .get(element_id.raw())
            .copied()
            .flatten()
    }

    pub(crate) fn m_stack_nodes(&mut self, m: usize) -> Vec<usize> {
        self.stack
            .split_off(self.stack.len() - m)
            .into_iter()
            .map(|entry| entry.0)
            .collect()
    }

    pub(crate) fn queue_mount_event(&mut self, id: ElementId) {
        self.queued_mounted_events.push(id);
    }
}

/// A writer for mutations that can be used with the RealDom.
pub struct MutationWriter<'a> {
    /// The realdom associated with this writer
    pub docm: DocumentMutator<'a>,
    /// The state associated with this writer
    pub state: &'a mut DioxusState,
}

impl<'a> MutationWriter<'a> {
    pub fn new(doc: &'a mut BaseDocument, state: &'a mut DioxusState) -> Self {
        MutationWriter {
            docm: doc.mutate(),
            state,
        }
    }
}

impl MutationWriter<'_> {
    /// Update an ElementId -> NodeId mapping
    fn set_id_mapping(&mut self, node_id: NodeId, element_id: ElementId) {
        let element_id: usize = element_id.raw();

        // Ensure node_id_mapping is large enough to contain element_id
        if self.state.node_id_mapping.len() <= element_id {
            self.state.node_id_mapping.resize(element_id + 1, None);
        }

        // Set the new mapping
        self.state.node_id_mapping[element_id] = Some(node_id);
    }

    /// Find a child in the document by child index path
    fn load_child(&self, path: &[u8]) -> NodeId {
        let top_of_stack_node_id = self.top_node();
        self.docm.node_at_path(top_of_stack_node_id, path)
    }

    fn top_node(&self) -> NodeId {
        self.state.stack.last().unwrap().0
    }

    fn top_element_id(&self) -> ElementId {
        self.state
            .stack
            .last()
            .unwrap()
            .1
            .expect("top node must be mapped to an ElementId")
    }
}

impl WriteMutations for MutationWriter<'_> {
    fn push_id(&mut self, id: ElementId) {
        trace!("push_id id:{}", id.raw());
        let node_id = self.state.element_to_node_id(id);
        self.state.stack.push((node_id, Some(id)));
    }

    fn pop_id(&mut self, id: ElementId) {
        trace!("pop_id id:{}", id.raw());
        let entry = self.state.stack.pop().unwrap();
        self.set_id_mapping(entry.0, id);
    }

    fn child(&mut self, index: usize) {
        trace!("child index:{index}");
        let child = self.load_child(&[index as u8]);
        *self.state.stack.last_mut().unwrap() = (child, None);
    }

    fn pop(&mut self) {
        trace!("pop");
        self.state.stack.pop();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        trace!("create_element tag:{tag} ns:{ns:?}");
        let node_id = self.docm.create_element(qual_name(tag, ns), Vec::new());
        self.state.stack.push((node_id, None));
    }

    fn create_text(&mut self, value: &str) {
        trace!("create_text text:{}", value);
        let node_id = self.docm.create_text_node(value);
        self.state.stack.push((node_id, None));
    }

    fn clone(&mut self) {
        trace!("clone");
        let node_id = self.top_node();
        *self.state.stack.last_mut().unwrap() = (self.docm.deep_clone_node(node_id), None);
    }

    fn append_children(&mut self, m: usize) {
        trace!("append_children m:{m}");
        let child_node_ids = self.state.m_stack_nodes(m);
        let parent_id = self.top_node();
        self.docm.append_children(parent_id, &child_node_ids);
    }

    fn replace_with(&mut self, m: usize) {
        trace!("replace_with m:{m}");
        let new_node_ids = self.state.m_stack_nodes(m);
        let target = self.state.stack.pop().unwrap();
        self.docm.replace_node_with(target.0, &new_node_ids);
    }

    fn insert_after(&mut self, m: usize) {
        trace!("insert_after m:{m}");
        let new_node_ids = self.state.m_stack_nodes(m);
        let anchor = self.top_node();
        self.docm.insert_nodes_after(anchor, &new_node_ids);
    }

    fn insert_before(&mut self, m: usize) {
        trace!("insert_before m:{m}");
        let new_node_ids = self.state.m_stack_nodes(m);
        let anchor = self.top_node();
        self.docm.insert_nodes_before(anchor, &new_node_ids);
    }

    fn remove(&mut self) {
        trace!("remove");
        let entry = self.state.stack.pop().unwrap();
        self.docm.remove_node(entry.0);
    }

    fn set_text(&mut self, value: &str) {
        trace!("set_text value:{}", value);
        let node_id = self.top_node();
        self.docm.set_node_text(node_id, value);
    }

    fn set_attribute(&mut self, local_name: &str, ns: Option<&str>, value: &AttributeValue) {
        let node_id = self.top_node();
        fn is_falsy(val: &AttributeValue) -> bool {
            match val {
                AttributeValue::None => true,
                AttributeValue::Text(val) => val == "false",
                AttributeValue::Bool(val) => !val,
                AttributeValue::Int(val) => *val == 0,
                AttributeValue::Float(val) => *val == 0.0,
                _ => false,
            }
        }

        // Set/unset subdocument for <web-view __webview_document>
        if local_name == "__webview_document" {
            match value {
                AttributeValue::Any(value) => {
                    if let Some(value) = value
                        .as_any()
                        .downcast_ref::<WriteOnceAttr<Box<PlainDocument>>>()
                        && let Some(mut sub_document) = value.take()
                    {
                        sub_document
                            .inner_mut()
                            .set_shell_provider(self.docm.doc.shell_provider.clone());
                        self.docm.set_sub_document(node_id, sub_document);
                    }
                }
                _ => self.docm.remove_sub_document(node_id),
            }
        }

        // Set/unset custom widget for <object data>
        if local_name == "data" {
            let element_name = self.docm.element_name(node_id).unwrap();
            if element_name.local.as_ref() == "object" {
                match value {
                    AttributeValue::Any(value) => {
                        if let Some(value) = value
                            .as_any()
                            .downcast_ref::<WriteOnceAttr<Box<dyn Widget>>>()
                            && let Some(widget) = value.take()
                        {
                            self.docm.set_custom_widget(node_id, widget);
                        }
                    }
                    _ => self.docm.remove_custom_widget(node_id),
                }
            }
        }

        let falsy = is_falsy(value);
        match value {
            AttributeValue::None => {
                set_attribute_inner(&mut self.docm, local_name, ns, None, falsy, node_id)
            }
            AttributeValue::Text(value) => {
                set_attribute_inner(&mut self.docm, local_name, ns, Some(value), falsy, node_id)
            }
            AttributeValue::Float(value) => {
                let value = value.to_string();
                set_attribute_inner(&mut self.docm, local_name, ns, Some(&value), falsy, node_id);
            }
            AttributeValue::Int(value) => {
                let value = value.to_string();
                set_attribute_inner(&mut self.docm, local_name, ns, Some(&value), falsy, node_id);
            }
            AttributeValue::Bool(value) => {
                let value = value.to_string();
                set_attribute_inner(&mut self.docm, local_name, ns, Some(&value), falsy, node_id);
            }
            _ => {
                // FIXME: support all attribute types
            }
        };
    }

    fn add_event_listener(&mut self, name: &str) {
        let id = self.top_element_id();
        // Mounted events are fired immediately after the element is mounted.
        if name == "mounted" {
            self.state.queue_mount_event(id);
            return;
        }

        // We're going to actually set the listener here as a placeholder - in JS this would also be a placeholder
        // we might actually just want to attach the attribute to the root element (delegation)
        let value = AttributeValue::Text("<rust func>".into());
        self.set_attribute(name, None, &value);

        // Also set the data-dioxus-id attribute so we can find the element later
        let value = AttributeValue::Text(id.raw().to_string());
        self.set_attribute("data-dioxus-id", None, &value);

        // node.add_event_listener(name);

        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.state.event_handler_counts[idx] += 1;
        }
    }

    fn remove_event_listener(&mut self, name: &str) {
        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.state.event_handler_counts[idx] -= 1;
        }
    }
}

fn set_attribute_inner(
    docm: &mut DocumentMutator<'_>,
    local_name: &str,
    ns: Option<&str>,
    value: Option<&str>,
    is_falsy: bool,
    node_id: usize,
) {
    trace!("set_attribute node_id:{node_id} ns: {ns:?} name:{local_name}, value:{value:?}");

    // Dioxus has overloaded the style namespace to accumulate style attributes without a `style` block
    // TODO: accumulate style attributes into a single style element.
    if ns == Some("style") {
        match value {
            Some(value) => docm.set_style_property(node_id, local_name, value),
            None => docm.remove_style_property(node_id, local_name),
        }
        return;
    }

    let name = qual_name(local_name, ns);

    // FIXME: more principled handling of special case attributes
    match value {
        None => docm.clear_attribute(node_id, name),
        Some(value) => {
            if local_name == "checked" && is_falsy {
                docm.clear_attribute(node_id, name);
            } else if local_name == "dangerous_inner_html" {
                docm.set_inner_html(node_id, value);
            } else {
                docm.set_attribute(node_id, name, value);
            }
        }
    }
}
