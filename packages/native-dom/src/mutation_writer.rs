//! Integration between Dioxus and Blitz
use crate::{qual_name, trace, NodeId};
use blitz_dom::{BaseDocument, DocumentMutator};
use blitz_traits::events::DomEventKind;
use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use rustc_hash::FxHashMap;
use std::str::FromStr as _;

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Store of templates keyed by unique name
    pub(crate) templates: FxHashMap<Template, Vec<NodeId>>,
    /// Stack machine state for applying dioxus mutations
    pub(crate) stack: Vec<NodeId>,
    /// Mapping from vdom ElementId -> rdom NodeId
    pub(crate) node_id_mapping: Vec<Option<NodeId>>,
    /// Count of each handler type
    pub(crate) event_handler_counts: [u32; 32],
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create(root_id: usize) -> Self {
        Self {
            templates: FxHashMap::default(),
            stack: vec![root_id],
            node_id_mapping: vec![Some(root_id)],
            event_handler_counts: [0; 32],
        }
    }

    /// Convert an ElementId to a NodeId
    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.try_element_to_node_id(element_id).unwrap()
    }

    /// Attempt to convert an ElementId to a NodeId. This will return None if the ElementId is not in the RealDom.
    pub fn try_element_to_node_id(&self, element_id: ElementId) -> Option<NodeId> {
        self.node_id_mapping.get(element_id.0).copied().flatten()
    }

    pub(crate) fn anchor_and_nodes(&mut self, id: ElementId, m: usize) -> (usize, Vec<usize>) {
        let anchor_node_id = self.element_to_node_id(id);
        let new_nodes = self.m_stack_nodes(m);
        (anchor_node_id, new_nodes)
    }

    pub(crate) fn m_stack_nodes(&mut self, m: usize) -> Vec<usize> {
        self.stack.split_off(self.stack.len() - m)
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
        let element_id: usize = element_id.0;

        // Ensure node_id_mapping is large enough to contain element_id
        if self.state.node_id_mapping.len() <= element_id {
            self.state.node_id_mapping.resize(element_id + 1, None);
        }

        // Set the new mapping
        self.state.node_id_mapping[element_id] = Some(node_id);
    }

    /// Create a ElementId -> NodeId mapping and push the node to the stack
    fn map_new_node(&mut self, node_id: NodeId, element_id: ElementId) {
        self.set_id_mapping(node_id, element_id);
        self.state.stack.push(node_id);
    }

    /// Find a child in the document by child index path
    fn load_child(&self, path: &[u8]) -> NodeId {
        let top_of_stack_node_id = *self.state.stack.last().unwrap();
        self.docm.node_at_path(top_of_stack_node_id, path)
    }
}

impl WriteMutations for MutationWriter<'_> {
    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        trace!("assign_node_id path:{:?} id:{}", path, id.0);

        // If there is an existing node already mapped to that ID and it has no parent, then drop it
        // TODO: more automated GC/ref-counted semantics for node lifetimes
        if let Some(node_id) = self.state.try_element_to_node_id(id) {
            self.docm.remove_node_if_unparented(node_id);
        }

        // Map the node at specified path
        self.set_id_mapping(self.load_child(path), id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        trace!("create_placeholder id:{}", id.0);
        let node_id = self.docm.create_comment_node();
        self.map_new_node(node_id, id);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        trace!("create_text_node id:{} text:{}", id.0, value);
        let node_id = self.docm.create_text_node(value);
        self.map_new_node(node_id, id);
    }

    fn append_children(&mut self, id: ElementId, m: usize) {
        trace!("append_children id:{} m:{}", id.0, m);
        let (parent_id, child_node_ids) = self.state.anchor_and_nodes(id, m);
        self.docm.append_children(parent_id, &child_node_ids);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        trace!("insert_nodes_after id:{} m:{}", id.0, m);
        let (anchor_node_id, new_node_ids) = self.state.anchor_and_nodes(id, m);
        self.docm.insert_nodes_after(anchor_node_id, &new_node_ids);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        trace!("insert_nodes_before id:{} m:{}", id.0, m);
        let (anchor_node_id, new_node_ids) = self.state.anchor_and_nodes(id, m);
        self.docm.insert_nodes_before(anchor_node_id, &new_node_ids);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        trace!("replace_node_with id:{} m:{}", id.0, m);
        let (anchor_node_id, new_node_ids) = self.state.anchor_and_nodes(id, m);
        self.docm.replace_node_with(anchor_node_id, &new_node_ids);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        trace!("replace_placeholder_with_nodes path:{:?} m:{}", path, m);
        // WARNING: DO NOT REORDER
        // The order of the following two lines is very important as "m_stack_nodes" mutates
        // the stack and then "load_child" reads from the top of the stack.
        let new_node_ids = self.state.m_stack_nodes(m);
        let anchor_node_id = self.load_child(path);
        self.docm.replace_node_with(anchor_node_id, &new_node_ids);
    }

    fn remove_node(&mut self, id: ElementId) {
        trace!("remove_node id:{}", id.0);
        let node_id = self.state.element_to_node_id(id);
        self.docm.remove_node(node_id);
    }

    fn push_root(&mut self, id: ElementId) {
        trace!("push_root id:{}", id.0);
        let node_id = self.state.element_to_node_id(id);
        self.state.stack.push(node_id);
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        trace!("set_node_text id:{} value:{}", id.0, value);
        let node_id = self.state.element_to_node_id(id);
        self.docm.set_node_text(node_id, value);
    }

    fn set_attribute(
        &mut self,
        local_name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let node_id = self.state.element_to_node_id(id);
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

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        // TODO: proper template node support
        let template_entry = self.state.templates.entry(template).or_insert_with(|| {
            let template_root_ids: Vec<NodeId> = template
                .roots
                .iter()
                .map(|root| create_template_node(&mut self.docm, root))
                .collect();

            template_root_ids
        });

        let template_node_id = template_entry[index];
        let clone_id = self.docm.deep_clone_node(template_node_id);

        trace!("load_template template_node_id:{template_node_id} clone_id:{clone_id}");
        self.map_new_node(clone_id, id);
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        // We're going to actually set the listener here as a placeholder - in JS this would also be a placeholder
        // we might actually just want to attach the attribute to the root element (delegation)
        let value = AttributeValue::Text("<rust func>".into());
        self.set_attribute(name, None, &value, id);

        // Also set the data-dioxus-id attribute so we can find the element later
        let value = AttributeValue::Text(id.0.to_string());
        self.set_attribute("data-dioxus-id", None, &value, id);

        // node.add_event_listener(name);

        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.state.event_handler_counts[idx] += 1;
        }
    }

    fn remove_event_listener(&mut self, name: &'static str, _id: ElementId) {
        if let Ok(kind) = DomEventKind::from_str(name) {
            let idx = kind.discriminant() as usize;
            self.state.event_handler_counts[idx] -= 1;
        }
    }
}

fn create_template_node(docm: &mut DocumentMutator<'_>, node: &TemplateNode) -> NodeId {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let name = qual_name(tag, *namespace);
            // let attrs = attrs.iter().filter_map(map_template_attr).collect();
            let node_id = docm.create_element(name, Vec::new());

            for attr in attrs.iter() {
                let TemplateAttribute::Static {
                    name,
                    value,
                    namespace,
                } = attr
                else {
                    continue;
                };
                let falsy = *value == "false";
                set_attribute_inner(docm, name, *namespace, Some(value), falsy, node_id);
            }

            let child_ids: Vec<NodeId> = children
                .iter()
                .map(|child| create_template_node(docm, child))
                .collect();

            docm.append_children(node_id, &child_ids);

            node_id
        }
        TemplateNode::Text { text } => docm.create_text_node(text),
        TemplateNode::Dynamic { .. } => docm.create_comment_node(),
    }
}

fn set_attribute_inner(
    docm: &mut DocumentMutator<'_>,
    local_name: &'static str,
    ns: Option<&'static str>,
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
