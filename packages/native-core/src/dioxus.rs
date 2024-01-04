//! Integration between Dioxus and the RealDom

use crate::tree::TreeMut;
use dioxus_core::{BorrowedAttributeValue, ElementId, Mutations, TemplateNode};
use rustc_hash::{FxHashMap, FxHashSet};
use shipyard::Component;

use crate::{
    node::{
        ElementNode, FromAnyValue, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
        TextNode,
    },
    prelude::*,
    real_dom::NodeTypeMut,
    NodeId,
};

#[derive(Component)]
struct ElementIdComponent(ElementId);

/// The state of the Dioxus integration with the RealDom
pub struct DioxusState {
    templates: FxHashMap<String, Vec<NodeId>>,
    stack: Vec<NodeId>,
    node_id_mapping: Vec<Option<NodeId>>,
}

impl DioxusState {
    /// Initialize the DioxusState in the RealDom
    pub fn create<V: FromAnyValue + Send + Sync>(rdom: &mut RealDom<V>) -> Self {
        let root_id = rdom.root_id();
        let mut root = rdom.get_mut(root_id).unwrap();
        root.insert(ElementIdComponent(ElementId(0)));
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

    fn set_element_id<V: FromAnyValue + Send + Sync>(
        &mut self,
        mut node: NodeMut<V>,
        element_id: ElementId,
    ) {
        let node_id = node.id();
        node.insert(ElementIdComponent(element_id));
        if self.node_id_mapping.len() <= element_id.0 {
            self.node_id_mapping.resize(element_id.0 + 1, None);
        } else if let Some(mut node) =
            self.node_id_mapping[element_id.0].and_then(|id| node.real_dom_mut().get_mut(id))
        {
            node.remove();
        }

        self.node_id_mapping[element_id.0] = Some(node_id);
    }

    fn load_child<V: FromAnyValue + Send + Sync>(&self, rdom: &RealDom<V>, path: &[u8]) -> NodeId {
        let mut current = rdom.get(*self.stack.last().unwrap()).unwrap();
        for i in path {
            let new_id = current.child_ids()[*i as usize];
            current = rdom.get(new_id).unwrap();
        }
        current.id()
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations<V: FromAnyValue + Send + Sync>(
        &mut self,
        rdom: &mut RealDom<V>,
        mutations: Mutations,
    ) {
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
                    let node_type_mut = node.node_type_mut();
                    if let NodeTypeMut::Text(mut text) = node_type_mut {
                        *text.text_mut() = value.to_string();
                    } else {
                        drop(node_type_mut);
                        node.set_type(NodeType::Text(TextNode {
                            text: value.to_string(),
                            listeners: FxHashSet::default(),
                        }));
                    }
                }
                LoadTemplate { name, index, id } => {
                    let template_id = self.templates[name][index];
                    let clone_id = rdom.get_mut(template_id).unwrap().clone_node();
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
                        rdom.tree_mut().insert_before(old_node_id, new);
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
                    let mut node_type_mut = node.node_type_mut();
                    if let NodeTypeMut::Element(element) = &mut node_type_mut {
                        if let BorrowedAttributeValue::None = &value {
                            element.remove_attribute(&OwnedAttributeDiscription {
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
                    let node_type_mut = node.node_type_mut();
                    if let NodeTypeMut::Text(mut text) = node_type_mut {
                        *text.text_mut() = value.to_string();
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

fn create_template_node<V: FromAnyValue + Send + Sync>(
    rdom: &mut RealDom<V>,
    node: &TemplateNode,
) -> NodeId {
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

/// A trait that extends the `NodeImmutable` trait with methods that are useful for dioxus.
pub trait NodeImmutableDioxusExt<V: FromAnyValue + Send + Sync>: NodeImmutable<V> {
    /// Returns the id of the element that this node is mounted to.
    /// Not all nodes are mounted to an element, only nodes with dynamic content that have been renderered will have an id.
    fn mounted_id(&self) -> Option<ElementId> {
        let id = self.get::<ElementIdComponent>();
        id.map(|id| id.0)
    }
}

impl<T: NodeImmutable<V>, V: FromAnyValue + Send + Sync> NodeImmutableDioxusExt<V> for T {}
