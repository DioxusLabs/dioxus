//! Integration between Dioxus and Blitz
use crate::{NodeId, qual_name, trace, write_once_attr::WriteOnceAttr};
use blitz_dom::{BaseDocument, Document as _, DocumentMutator, PlainDocument, Widget};
use blitz_traits::events::DomEventKind;
use dioxus_core::{AttributeValue, ElementId, WriteMutations};
use std::str::FromStr as _;

pub(crate) trait RealDom {
    type NodeId: Copy;

    fn create_element(&mut self, tag: &str, ns: Option<&str>) -> Self::NodeId;

    fn create_text(&mut self, value: &str) -> Self::NodeId;

    fn deep_clone(&mut self, node: Self::NodeId) -> Self::NodeId;

    fn nth_child(&mut self, parent: Self::NodeId, index: usize) -> Self::NodeId;

    fn append_children(&mut self, parent: Self::NodeId, children: &[Self::NodeId]);

    fn insert_after(&mut self, anchor: Self::NodeId, nodes: &[Self::NodeId]);

    fn insert_before(&mut self, anchor: Self::NodeId, nodes: &[Self::NodeId]);

    fn replace(&mut self, target: Self::NodeId, replacements: &[Self::NodeId]);

    fn remove(&mut self, node: Self::NodeId);

    fn set_attribute(
        &mut self,
        node: Self::NodeId,
        name: &str,
        ns: Option<&str>,
        value: &AttributeValue,
    );

    fn set_text(&mut self, node: Self::NodeId, value: &str);

    fn add_event_listener(&mut self, node: Self::NodeId, element_id: ElementId, name: &str);

    fn remove_event_listener(&mut self, node: Self::NodeId, element_id: ElementId, name: &str);
}

#[derive(Clone, Copy, Debug)]
struct StackEntry<N> {
    node: N,
    element_id: Option<ElementId>,
}

#[derive(Debug)]
struct StackState<N> {
    stack: Vec<StackEntry<N>>,
    element_to_node: Vec<Option<N>>,
}

impl<N: Copy> StackState<N> {
    fn new(root: N) -> Self {
        Self {
            stack: vec![StackEntry {
                node: root,
                element_id: Some(ElementId::ROOT),
            }],
            element_to_node: vec![Some(root)],
        }
    }

    fn element_to_node(&self, id: ElementId) -> Option<N> {
        self.element_to_node.get(id.raw()).copied().flatten()
    }

    fn lookup(&self, id: ElementId) -> N {
        self.element_to_node(id)
            .unwrap_or_else(|| panic!("renderer asked for unknown ElementId {}", id.raw()))
    }

    fn set_mapping(&mut self, id: ElementId, node: N) {
        let index = id.raw();
        if self.element_to_node.len() <= index {
            self.element_to_node.resize(index + 1, None);
        }
        self.element_to_node[index] = Some(node);
    }

    fn clear_mapping(&mut self, entry: StackEntry<N>) {
        if let Some(id) = entry.element_id
            && let Some(slot) = self.element_to_node.get_mut(id.raw())
        {
            *slot = None;
        }
    }

    fn push(&mut self, node: N, element_id: Option<ElementId>) {
        self.stack.push(StackEntry { node, element_id });
    }

    fn pop_entry(&mut self) -> StackEntry<N> {
        self.stack.pop().expect("renderer stack unexpectedly empty")
    }

    fn top(&self) -> StackEntry<N> {
        *self
            .stack
            .last()
            .expect("renderer stack unexpectedly empty")
    }

    fn replace_top(&mut self, node: N) {
        *self
            .stack
            .last_mut()
            .expect("renderer stack unexpectedly empty") = StackEntry {
            node,
            element_id: None,
        };
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<N> {
        let split = self.stack.len() - m;
        self.stack
            .split_off(split)
            .into_iter()
            .map(|entry| entry.node)
            .collect()
    }
}

pub(crate) struct StackWriter<'a, R: RealDom> {
    state: &'a mut StackState<R::NodeId>,
    backend: R,
}

impl<'a, R: RealDom> StackWriter<'a, R> {
    fn new(state: &'a mut StackState<R::NodeId>, backend: R) -> Self {
        Self { state, backend }
    }
}

impl<R: RealDom> WriteMutations for StackWriter<'_, R> {
    fn push_id(&mut self, id: ElementId) {
        let node = self.state.lookup(id);
        self.state.push(node, Some(id));
    }

    fn set_id(&mut self, id: ElementId) {
        let node = self.state.top().node;
        self.state.set_mapping(id, node);
    }

    fn child(&mut self, index: usize) {
        let parent = self.state.top().node;
        let child = self.backend.nth_child(parent, index);
        self.state.replace_top(child);
    }

    fn pop(&mut self) {
        self.state.pop_entry();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        let node = self.backend.create_element(tag, ns);
        self.state.push(node, None);
    }

    fn create_text(&mut self, value: &str) {
        let node = self.backend.create_text(value);
        self.state.push(node, None);
    }

    fn clone(&mut self) {
        let node = self.state.top().node;
        let cloned = self.backend.deep_clone(node);
        self.state.replace_top(cloned);
    }

    fn append_children(&mut self, m: usize) {
        let children = self.state.pop_nodes(m);
        let parent = self.state.top().node;
        self.backend.append_children(parent, &children);
    }

    fn replace_with(&mut self, m: usize) {
        let replacements = self.state.pop_nodes(m);
        let target = self.state.pop_entry();
        self.backend.replace(target.node, &replacements);
        self.state.clear_mapping(target);
    }

    fn insert_after(&mut self, m: usize) {
        let nodes = self.state.pop_nodes(m);
        let anchor = self.state.top().node;
        self.backend.insert_after(anchor, &nodes);
    }

    fn insert_before(&mut self, m: usize) {
        let nodes = self.state.pop_nodes(m);
        let anchor = self.state.top().node;
        self.backend.insert_before(anchor, &nodes);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        let node = self.state.top().node;
        self.backend.set_attribute(node, name, ns, value);
    }

    fn set_text(&mut self, value: &str) {
        let node = self.state.top().node;
        self.backend.set_text(node, value);
    }

    fn add_event_listener(&mut self, name: &str) {
        let entry = self.state.top();
        let element_id = entry
            .element_id
            .expect("event listener target must be mapped to an ElementId");
        self.backend
            .add_event_listener(entry.node, element_id, name);
    }

    fn remove_event_listener(&mut self, name: &str) {
        let entry = self.state.top();
        let element_id = entry
            .element_id
            .expect("event listener target must be mapped to an ElementId");
        self.backend
            .remove_event_listener(entry.node, element_id, name);
    }

    fn remove(&mut self) {
        let entry = self.state.pop_entry();
        self.backend.remove(entry.node);
        self.state.clear_mapping(entry);
    }
}

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Stack machine state for applying dioxus mutations
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
    /// backend from the local stack machine.
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
/// The stack-machine bookkeeping lives in [`StackWriter`]; this renderer only
/// supplies the real tree semantics via [`BlitzBackend`].
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
        self.docm.child_ids(parent)[index]
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
