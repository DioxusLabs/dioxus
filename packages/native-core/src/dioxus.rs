use dioxus_core::{BorrowedAttributeValue, ElementId, Mutations, TemplateNode};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    node::{
        ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
        TextNode,
    },
    prelude::NodeImmutable,
    real_dom::NodeTypeMut,
    NodeId, NodeMut, RealDom,
};

pub struct DioxusState {
    templates: FxHashMap<String, Vec<NodeId>>,
    stack: Vec<NodeId>,
    node_id_mapping: Vec<Option<NodeId>>,
}

impl DioxusState {
    pub fn create(rdom: &mut RealDom) -> Self {
        rdom.insert_slab::<ElementId>();
        let root_id = rdom.root_id();
        let mut root = rdom.get_mut(root_id).unwrap();
        root.insert(ElementId(0));
        Self {
            templates: FxHashMap::default(),
            stack: vec![root_id],
            node_id_mapping: vec![Some(root_id)],
        }
    }

    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.node_id_mapping.get(element_id.0).unwrap().unwrap()
    }

    fn set_element_id(&mut self, mut node: NodeMut, element_id: ElementId) {
        let node_id = node.id();
        node.insert(element_id);
        if self.node_id_mapping.len() <= element_id.0 {
            self.node_id_mapping.resize(element_id.0 + 1, None);
        }
        self.node_id_mapping[element_id.0] = Some(node_id);
    }

    pub fn load_child(&self, rdom: &RealDom, path: &[u8]) -> NodeId {
        let mut current = rdom.get(*self.stack.last().unwrap()).unwrap();
        for i in path {
            let new_id = current.child_ids().unwrap()[*i as usize];
            current = rdom.get(new_id).unwrap();
        }
        current.id()
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(&mut self, rdom: &mut RealDom, mutations: Mutations) {
        for template in mutations.templates {
            let mut template_root_ids = Vec::new();
            for root in template.roots {
                let id = create_template_node(rdom, root);
                template_root_ids.push(id);
            }
            self.templates
                .insert(template.name.to_string(), template_root_ids);
        }

        for e in mutations.edits {
            use dioxus_core::Mutation::*;
            match e {
                AppendChildren { id, m } => {
                    let children = self.stack.split_off(self.stack.len() - m);
                    let parent = self.element_to_node_id(id);
                    for child in children {
                        rdom.get_mut(parent).unwrap().add_child(child);
                    }
                }
                AssignId { path, id } => {
                    let node_id = self.load_child(rdom, path);
                    self.set_element_id(rdom.get_mut(node_id).unwrap(), id);
                }
                CreatePlaceholder { id } => {
                    let node = NodeType::Placeholder;
                    let node = rdom.create_node(node);
                    let node_id = node.id();
                    self.set_element_id(node, id);
                    self.stack.push(node_id);
                }
                CreateTextNode { value, id } => {
                    let node_data = NodeType::Text(TextNode {
                        listeners: FxHashSet::default(),
                        text: value.to_string(),
                    });
                    let node = rdom.create_node(node_data);
                    let node_id = node.id();
                    self.set_element_id(node, id);
                    self.stack.push(node_id);
                }
                HydrateText { path, value, id } => {
                    let node_id = self.load_child(rdom, path);
                    let node = rdom.get_mut(node_id).unwrap();
                    self.set_element_id(node, id);
                    let mut node = rdom.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Text(text) = node.node_type_mut() {
                        *text = value.to_string();
                    } else {
                        node.set_type(NodeType::Text(TextNode {
                            text: value.to_string(),
                            listeners: FxHashSet::default(),
                        }));
                    }
                }
                LoadTemplate { name, index, id } => {
                    let template_id = self.templates[name][index];
                    let clone_id = rdom.clone_node(template_id);
                    let clone = rdom.get_mut(clone_id).unwrap();
                    self.set_element_id(clone, id);
                    self.stack.push(clone_id);
                }
                ReplaceWith { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        let mut node = rdom.get_mut(new).unwrap();
                        node.insert_before(old_node_id);
                    }
                    rdom.get_mut(old_node_id).unwrap().remove();
                }
                ReplacePlaceholder { path, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.load_child(rdom, path);
                    for new in new_nodes {
                        let mut node = rdom.get_mut(new).unwrap();
                        node.insert_before(old_node_id);
                    }
                    rdom.get_mut(old_node_id).unwrap().remove();
                }
                InsertAfter { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes.into_iter().rev() {
                        let mut node = rdom.get_mut(new).unwrap();
                        node.insert_after(old_node_id);
                    }
                }
                InsertBefore { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        rdom.tree.insert_before(old_node_id, new);
                    }
                }
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = rdom.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(element) = &mut node.node_type_mut() {
                        if let BorrowedAttributeValue::None = &value {
                            element.remove_attributes(&OwnedAttributeDiscription {
                                name: name.to_string(),
                                namespace: ns.map(|s| s.to_string()),
                            });
                        } else {
                            element.set_attribute(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: ns.map(|s| s.to_string()),
                                },
                                OwnedAttributeValue::from(value),
                            );
                        }
                    }
                }
                SetText { value, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = rdom.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Text(text) = node.node_type_mut() {
                        *text = value.to_string();
                    }
                }
                NewEventListener { name, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = rdom.get_mut(node_id).unwrap();
                    node.add_event_listener(name);
                }
                RemoveEventListener { id, name } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = rdom.get_mut(node_id).unwrap();
                    node.remove_event_listener(name);
                }
                Remove { id } => {
                    let node_id = self.element_to_node_id(id);
                    rdom.get_mut(node_id).unwrap().remove();
                }
                PushRoot { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.stack.push(node_id);
                }
            }
        }
    }
}

fn create_template_node(rdom: &mut RealDom, node: &TemplateNode) -> NodeId {
    match node {
        TemplateNode::Element {
            tag,
            namespace,
            attrs,
            children,
        } => {
            let node = NodeType::Element(ElementNode {
                tag: tag.to_string(),
                namespace: namespace.map(|s| s.to_string()),
                attributes: attrs
                    .iter()
                    .filter_map(|attr| match attr {
                        dioxus_core::TemplateAttribute::Static {
                            name,
                            value,
                            namespace,
                        } => Some((
                            OwnedAttributeDiscription {
                                namespace: namespace.map(|s| s.to_string()),
                                name: name.to_string(),
                            },
                            OwnedAttributeValue::Text(value.to_string()),
                        )),
                        dioxus_core::TemplateAttribute::Dynamic { .. } => None,
                    })
                    .collect(),
                listeners: FxHashSet::default(),
            });
            let node_id = rdom.create_node(node).id();
            for child in *children {
                let child_id = create_template_node(rdom, child);
                rdom.get_mut(node_id).unwrap().add_child(child_id);
            }
            node_id
        }
        TemplateNode::Text { text } => rdom
            .create_node(NodeType::Text(TextNode {
                text: text.to_string(),
                ..Default::default()
            }))
            .id(),
        TemplateNode::Dynamic { .. } => rdom.create_node(NodeType::Placeholder).id(),
        TemplateNode::DynamicText { .. } => {
            rdom.create_node(NodeType::Text(TextNode::default())).id()
        }
    }
}

pub trait NodeImmutableDioxusExt<V: FromAnyValue + Send + Sync>: NodeImmutable<V> {
    fn mounted_id(&self) -> Option<ElementId> {
        self.get().copied()
    }
}

impl<T: NodeImmutable<V>, V: FromAnyValue + Send + Sync> NodeImmutableDioxusExt<V> for T {}
