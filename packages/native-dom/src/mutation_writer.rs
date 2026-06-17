//! Integration between Dioxus and Blitz
use crate::{NodeId, qual_name, trace, write_once_attr::WriteOnceAttr};
use blitz_dom::{BaseDocument, Document as _, DocumentMutator, PlainDocument, Widget};
use blitz_traits::events::DomEventKind;
use dioxus_core::{AttributeValue, ElementId};
use dioxus_stack::{RealDom, StackState, StackWriter};
use std::str::FromStr as _;

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Stack machine state for applying dioxus mutations (owned by core)
    stack: StackState<NodeId>,
    /// Count of each handler type
    pub(crate) event_handler_counts: [u32; 32],
    /// Mounted events queued as elements are mounted
    pub(crate) queued_mounted_events: Vec<ElementId>,
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create(root_id: usize) -> Self {
        Self {
            stack: StackState::new(root_id),
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
        self.stack.element_to_node(element_id)
    }

    /// Build a writer that applies dioxus mutations to `doc`, driving the blitz
    /// backend from the core-owned stack machine.
    pub(crate) fn writer<'a>(&'a mut self, doc: &'a mut BaseDocument) -> MutationWriter<'a> {
        let Self {
            stack,
            event_handler_counts,
            queued_mounted_events,
        } = self;
        StackWriter::new(
            stack,
            BlitzBackend {
                docm: doc.mutate(),
                event_handler_counts,
                queued_mounted_events,
            },
        )
    }
}

/// A writer for mutations that can be used with the RealDom.
///
/// The stack-machine bookkeeping lives in core's [`StackWriter`]; this renderer
/// only supplies the real tree semantics via [`BlitzBackend`].
pub(crate) type MutationWriter<'a> = StackWriter<'a, BlitzBackend<'a>>;

/// The blitz-backed "real semantics" for the dioxus stack machine.
pub(crate) struct BlitzBackend<'a> {
    /// The realdom mutation handle associated with this writer
    docm: DocumentMutator<'a>,
    /// Count of each handler type, kept so event dispatch can skip unused kinds
    event_handler_counts: &'a mut [u32; 32],
    /// Mounted events queued as elements are mounted
    queued_mounted_events: &'a mut Vec<ElementId>,
}

impl RealDom for BlitzBackend<'_> {
    type NodeId = NodeId;

    fn create_element(&mut self, tag: &str, ns: Option<&str>) -> NodeId {
        trace!("create_element tag:{tag} ns:{ns:?}");
        self.docm.create_element(qual_name(tag, ns), Vec::new())
    }

    fn create_text(&mut self, value: &str) -> NodeId {
        trace!("create_text text:{value}");
        self.docm.create_text_node(value)
    }

    fn deep_clone(&mut self, node: NodeId) -> NodeId {
        trace!("clone node:{node}");
        self.docm.deep_clone_node(node)
    }

    fn nth_child(&mut self, parent: NodeId, index: usize) -> NodeId {
        trace!("nth_child parent:{parent} index:{index}");
        self.docm.node_at_path(parent, &[index as u8])
    }

    fn append_children(&mut self, parent: NodeId, children: &[NodeId]) {
        trace!("append_children parent:{parent} children:{children:?}");
        self.docm.append_children(parent, children);
    }

    fn insert_after(&mut self, anchor: NodeId, nodes: &[NodeId]) {
        trace!("insert_after anchor:{anchor} nodes:{nodes:?}");
        self.docm.insert_nodes_after(anchor, nodes);
    }

    fn insert_before(&mut self, anchor: NodeId, nodes: &[NodeId]) {
        trace!("insert_before anchor:{anchor} nodes:{nodes:?}");
        self.docm.insert_nodes_before(anchor, nodes);
    }

    fn replace(&mut self, target: NodeId, replacements: &[NodeId]) {
        trace!("replace target:{target} replacements:{replacements:?}");
        self.docm.replace_node_with(target, replacements);
    }

    fn remove(&mut self, node: NodeId) {
        trace!("remove node:{node}");
        self.docm.remove_node(node);
    }

    fn set_attribute(
        &mut self,
        node: NodeId,
        name: &str,
        ns: Option<&str>,
        value: &AttributeValue,
    ) {
        self.set_attribute_impl(node, name, ns, value);
    }

    fn set_text(&mut self, node: NodeId, value: &str) {
        trace!("set_text node:{node} value:{value}");
        self.docm.set_node_text(node, value);
    }

    fn add_event_listener(&mut self, node: NodeId, element_id: ElementId, name: &str) {
        // Mounted events are fired immediately after the element is mounted.
        if name == "mounted" {
            self.queued_mounted_events.push(element_id);
            return;
        }

        // We're going to actually set the listener here as a placeholder - in JS this would also be a placeholder
        // we might actually just want to attach the attribute to the root element (delegation)
        let value = AttributeValue::Text("<rust func>".into());
        self.set_attribute_impl(node, name, None, &value);

        // Also set the data-dioxus-id attribute so we can find the element later
        let value = AttributeValue::Text(element_id.raw().to_string());
        self.set_attribute_impl(node, "data-dioxus-id", None, &value);

        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.event_handler_counts[idx] += 1;
        }
    }

    fn remove_event_listener(&mut self, _node: NodeId, _element_id: ElementId, name: &str) {
        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.event_handler_counts[idx] -= 1;
        }
    }
}

impl BlitzBackend<'_> {
    fn set_attribute_impl(
        &mut self,
        node_id: NodeId,
        local_name: &str,
        ns: Option<&str>,
        value: &AttributeValue,
    ) {
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
