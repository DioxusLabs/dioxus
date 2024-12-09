use crate::{dioxus_document::qual_name, NodeId};
use blitz_dom::{
    local_name, namespace_url,
    node::{Attribute, NodeSpecificData},
    ns, Document, ElementNodeData, NodeData, QualName, RestyleHint,
};
use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use rustc_hash::FxHashMap;
use std::collections::HashSet;

/// The state of the Dioxus integration with the RealDom
#[derive(Debug)]
pub struct DioxusState {
    /// Store of templates keyed by unique name
    templates: FxHashMap<Template, Vec<NodeId>>,
    /// Stack machine state for applying dioxus mutations
    stack: Vec<NodeId>,
    /// Mapping from vdom ElementId -> rdom NodeId
    node_id_mapping: Vec<Option<NodeId>>,
}

/// A writer for mutations that can be used with the RealDom.
pub struct MutationWriter<'a> {
    /// The realdom associated with this writer
    pub doc: &'a mut Document,

    /// The state associated with this writer
    pub state: &'a mut DioxusState,

    pub style_nodes: HashSet<usize>,
}

impl<'a> MutationWriter<'a> {
    pub fn new(doc: &'a mut Document, state: &'a mut DioxusState) -> Self {
        MutationWriter {
            doc,
            state,
            style_nodes: HashSet::new(),
        }
    }

    fn is_style_node(&self, node_id: NodeId) -> bool {
        self.doc
            .get_node(node_id)
            .unwrap()
            .raw_dom_data
            .is_element_with_tag_name(&local_name!("style"))
    }

    fn maybe_push_style_node(&mut self, node_id: impl Into<Option<NodeId>>) {
        if let Some(node_id) = node_id.into() {
            if self.is_style_node(node_id) {
                self.style_nodes.insert(node_id);
            }
        }
    }

    #[track_caller]
    fn maybe_push_parent_style_node(&mut self, node_id: NodeId) {
        let parent_id = self.doc.get_node(node_id).unwrap().parent;
        self.maybe_push_style_node(parent_id);
    }
}

impl Drop for MutationWriter<'_> {
    fn drop(&mut self) {
        // Add/Update inline stylesheets (<style> elements)
        for &id in &self.style_nodes {
            self.doc.upsert_stylesheet_for_node(id);
        }
    }
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create(doc: &mut Document) -> Self {
        let root = doc.root_element();
        let root_id = root.id;

        Self {
            templates: FxHashMap::default(),
            stack: vec![root_id],
            node_id_mapping: vec![Some(root_id)],
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

    // /// Create a mutation writer for the RealDom
    // pub fn create_mutation_writer<'a>(&'a mut self, doc: &'a mut Document) -> MutationWriter<'a> {
    //     MutationWriter { doc, state: self }
    // }
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

    /// Find a child in the document by child index path
    fn load_child(&self, path: &[u8]) -> NodeId {
        let mut current = self
            .doc
            .get_node(*self.state.stack.last().unwrap())
            .unwrap();
        for i in path {
            let new_id = current.children[*i as usize];
            current = self.doc.get_node(new_id).unwrap();
        }
        current.id
    }
}

impl WriteMutations for MutationWriter<'_> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("append_children id:{} m:{}", id.0, m);

        let children = self.state.stack.split_off(self.state.stack.len() - m);
        let parent = self.state.element_to_node_id(id);
        for child in children {
            self.doc.get_node_mut(parent).unwrap().children.push(child);
            self.doc.get_node_mut(child).unwrap().parent = Some(parent);
        }

        self.maybe_push_style_node(parent);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("assign_node_id path:{:?} id:{}", path, id.0);

        // If there is an existing node already mapped to that ID and
        // it has no parent, then drop it
        if let Some(node_id) = self.state.try_element_to_node_id(id) {
            if let Some(node) = self.doc.get_node(node_id) {
                if node.parent.is_none() {
                    self.doc.remove_and_drop_node(node_id);
                }
            }
        }

        let node_id = self.load_child(path);
        self.set_id_mapping(node_id, id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("create_placeholder id:{}", id.0);

        let node_id = self.doc.create_node(NodeData::Comment);
        self.set_id_mapping(node_id, id);
        self.state.stack.push(node_id);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("create_text_node id:{} text:{}", id.0, value);

        let node_id = self.doc.create_text_node(value);
        self.set_id_mapping(node_id, id);
        self.state.stack.push(node_id);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let template_entry = self.state.templates.entry(template).or_insert_with(|| {
            let template_root_ids: Vec<NodeId> = template
                .roots
                .iter()
                .map(|root| create_template_node(self.doc, root))
                .collect();

            template_root_ids
        });

        let template_node_id = template_entry[index];
        let clone_id = self.doc.deep_clone_node(template_node_id);
        self.set_id_mapping(clone_id, id);
        self.state.stack.push(clone_id);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("replace_node_with id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        self.maybe_push_parent_style_node(anchor_node_id);
        self.doc.insert_before(anchor_node_id, &new_nodes);
        self.doc.remove_node(anchor_node_id);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("replace_placeholder_with_nodes path:{:?} m:{}", path, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.load_child(path);
        self.maybe_push_parent_style_node(anchor_node_id);
        self.doc.insert_before(anchor_node_id, &new_nodes);
        self.doc.remove_node(anchor_node_id);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("insert_nodes_after id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        let next_sibling_id = self
            .doc
            .get_node(anchor_node_id)
            .unwrap()
            .forward(1)
            .map(|node| node.id);

        match next_sibling_id {
            Some(anchor_node_id) => {
                self.doc.insert_before(anchor_node_id, &new_nodes);
            }
            None => self.doc.append(anchor_node_id, &new_nodes),
        }

        self.maybe_push_parent_style_node(anchor_node_id);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        #[cfg(feature = "tracing")]
        tracing::info!("insert_nodes_before id:{} m:{}", id.0, m);

        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let anchor_node_id = self.state.element_to_node_id(id);
        self.doc.insert_before(anchor_node_id, &new_nodes);

        self.maybe_push_parent_style_node(anchor_node_id);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let node_id = self.state.element_to_node_id(id);

        #[cfg(feature = "tracing")]
        tracing::info!(
            "set_attribute node_id:{} ns: {:?} name:{}, value:{:?}",
            node_id,
            ns,
            name,
            value
        );

        self.doc.snapshot_node(node_id);

        let node = &mut self.doc.nodes[node_id];

        let stylo_element_data = &mut *node.stylo_element_data.borrow_mut();
        if let Some(data) = stylo_element_data {
            data.hint |= RestyleHint::restyle_subtree();
        }

        if let NodeData::Element(ref mut element) = node.raw_dom_data {
            if element.name.local == local_name!("input") && name == "checked" {
                set_input_checked_state(element, value);
            }
            // FIXME: support other non-text attributes
            else if let AttributeValue::Text(val) = value {
                if name == "value" {
                    // Update text input value
                    if let Some(input_data) = element.text_input_data_mut() {
                        input_data.set_text(&mut self.doc.font_ctx, &mut self.doc.layout_ctx, val);
                    }
                }

                // FIXME check namespace
                let existing_attr = element
                    .attrs
                    .iter_mut()
                    .find(|attr| attr.name.local == *name);

                if let Some(existing_attr) = existing_attr {
                    existing_attr.value.clear();
                    existing_attr.value.push_str(val);
                } else {
                    // we have overloaded the style namespace to accumulate style attributes without a `style` block
                    if ns == Some("style") {
                        // todo: need to accumulate style attributes into a single style
                        //
                        // element.
                    } else {
                        element.attrs.push(Attribute {
                            name: qual_name(name, ns),
                            value: val.to_string(),
                        });
                    }
                }
            }

            if let AttributeValue::None = value {
                // Update text input value
                if name == "value" {
                    if let Some(input_data) = element.text_input_data_mut() {
                        input_data.set_text(&mut self.doc.font_ctx, &mut self.doc.layout_ctx, "");
                    }
                }

                // FIXME: check namespace
                element.attrs.retain(|attr| attr.name.local != *name);
            }
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("set_node_text id:{} value:{}", id.0, value);

        let node_id = self.state.element_to_node_id(id);
        let node = self.doc.get_node_mut(node_id).unwrap();

        let text = match node.raw_dom_data {
            NodeData::Text(ref mut text) => text,
            // todo: otherwise this is basically element.textContent which is a bit different - need to parse as html
            _ => return,
        };

        let changed = text.content != value;
        if changed {
            text.content.clear();
            text.content.push_str(value);
            let parent = node.parent;
            self.maybe_push_style_node(parent);
        }
    }

    fn create_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // we're going to actually set the listener here as a placeholder - in JS this would also be a placeholder
        // we might actually just want to attach the attribute to the root element (delegation)
        self.set_attribute(
            _name,
            None,
            &AttributeValue::Text("<rust func>".into()),
            _id,
        );

        // also set the data-dioxus-id attribute so we can find the element later
        self.set_attribute(
            "data-dioxus-id",
            None,
            &AttributeValue::Text(_id.0.to_string()),
            _id,
        );

        // let node_id = self.state.element_to_node_id(id);
        // let mut node = self.rdom.get_mut(node_id).unwrap();
        // node.add_event_listener(name);
    }

    fn remove_event_listener(&mut self, _name: &'static str, _id: ElementId) {
        // let node_id = self.state.element_to_node_id(id);
        // let mut node = self.rdom.get_mut(node_id).unwrap();
        // node.remove_event_listener(name);
    }

    fn remove_node(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("remove_node id:{}", id.0);

        let node_id = self.state.element_to_node_id(id);
        self.doc.remove_node(node_id);
    }

    fn push_root(&mut self, id: ElementId) {
        #[cfg(feature = "tracing")]
        tracing::info!("push_root id:{}", id.0,);

        let node_id = self.state.element_to_node_id(id);
        self.state.stack.push(node_id);
    }
}

/// Set 'checked' state on an input based on given attributevalue
fn set_input_checked_state(element: &mut ElementNodeData, value: &AttributeValue) {
    let checked: bool;
    match value {
        AttributeValue::Bool(checked_bool) => {
            checked = *checked_bool;
        }
        AttributeValue::Text(val) => {
            if let Ok(checked_bool) = val.parse() {
                checked = checked_bool;
            } else {
                return;
            };
        }
        _ => {
            return;
        }
    };
    match element.node_specific_data {
        NodeSpecificData::CheckboxInput(ref mut checked_mut) => *checked_mut = checked,
        // If we have just constructed the element, set the node attribute,
        // and NodeSpecificData will be created from that later
        // this simulates the checked attribute being set in html,
        // and the element's checked property being set from that
        NodeSpecificData::None => element.attrs.push(Attribute {
            name: QualName {
                prefix: None,
                ns: ns!(html),
                local: local_name!("checked"),
            },
            value: checked.to_string(),
        }),
        _ => {}
    }
}

fn create_template_node(doc: &mut Document, node: &TemplateNode) -> NodeId {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let name = qual_name(tag, *namespace);
            let attrs = attrs
                .iter()
                .filter_map(|attr| match attr {
                    TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } => Some(Attribute {
                        name: qual_name(name, *namespace),
                        value: value.to_string(),
                    }),
                    TemplateAttribute::Dynamic { .. } => None,
                })
                .collect();

            let mut data = ElementNodeData::new(name, attrs);
            data.flush_style_attribute(doc.guard());

            let id = doc.create_node(NodeData::Element(data));
            let node = doc.get_node(id).unwrap();

            // Initialise style data
            *node.stylo_element_data.borrow_mut() = Some(Default::default());

            // If the node has an "id" attribute, store it in the ID map.
            // FIXME: implement
            // if let Some(id_attr) = node.attr(local_name!("id")) {
            //     doc.nodes_to_id.insert(id_attr.to_string(), id);
            // }

            let child_ids: Vec<NodeId> = children
                .iter()
                .map(|child| create_template_node(doc, child))
                .collect();
            for &child_id in &child_ids {
                doc.get_node_mut(child_id).unwrap().parent = Some(id);
            }
            doc.get_node_mut(id).unwrap().children = child_ids;

            id
        }
        TemplateNode::Text { text } => doc.create_text_node(text),
        TemplateNode::Dynamic { .. } => doc.create_node(NodeData::Comment),
    }
}
