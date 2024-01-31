//! Integration between Dioxus and the RealDom

use crate::tree::TreeMut;
use dioxus_core::{AttributeValue, ElementId, TemplateNode, WriteMutations};
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

    /// Create a mutation writer for the RealDom
    pub fn create_mutation_writer<'a, V: FromAnyValue + Send + Sync>(
        &'a mut self,
        rdom: &'a mut RealDom<V>,
    ) -> DioxusNativeCoreMutationWriter<'a, V> {
        DioxusNativeCoreMutationWriter { rdom, state: self }
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
}

/// A writer for mutations that can be used with the RealDom.
pub struct DioxusNativeCoreMutationWriter<'a, V: FromAnyValue + Send + Sync = ()> {
    /// The realdom associated with this writer
    pub rdom: &'a mut RealDom<V>,

    /// The state associated with this writer
    pub state: &'a mut DioxusState,
}

impl<V: FromAnyValue + Send + Sync> WriteMutations for DioxusNativeCoreMutationWriter<'_, V> {
    fn register_template(&mut self, template: dioxus_core::prelude::Template) {
        let mut template_root_ids = Vec::new();
        for root in template.roots {
            let id = create_template_node(self.rdom, root);
            template_root_ids.push(id);
        }
        self.state
            .templates
            .insert(template.name.to_string(), template_root_ids);
    }

    fn append_children(&mut self, id: ElementId, m: usize) {
        let children = self.state.stack.split_off(self.state.stack.len() - m);
        let parent = self.state.element_to_node_id(id);
        for child in children {
            self.rdom.get_mut(parent).unwrap().add_child(child);
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let node_id = self.state.load_child(self.rdom, path);
        self.state
            .set_element_id(self.rdom.get_mut(node_id).unwrap(), id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        let node = NodeType::Placeholder;
        let node = self.rdom.create_node(node);
        let node_id = node.id();
        self.state.set_element_id(node, id);
        self.state.stack.push(node_id);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        let node_data = NodeType::Text(TextNode {
            listeners: FxHashSet::default(),
            text: value.to_string(),
        });
        let node = self.rdom.create_node(node_data);
        let node_id = node.id();
        self.state.set_element_id(node, id);
        self.state.stack.push(node_id);
    }

    fn hydrate_text_node(&mut self, path: &'static [u8], value: &str, id: ElementId) {
        let node_id = self.state.load_child(self.rdom, path);
        let node = self.rdom.get_mut(node_id).unwrap();
        self.state.set_element_id(node, id);
        let mut node = self.rdom.get_mut(node_id).unwrap();
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

    fn load_template(&mut self, name: &'static str, index: usize, id: ElementId) {
        let template_id = self.state.templates[name][index];
        let clone_id = self.rdom.get_mut(template_id).unwrap().clone_node();
        let clone = self.rdom.get_mut(clone_id).unwrap();
        self.state.set_element_id(clone, id);
        self.state.stack.push(clone_id);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let old_node_id = self.state.element_to_node_id(id);
        for new in new_nodes {
            let mut node = self.rdom.get_mut(new).unwrap();
            node.insert_before(old_node_id);
        }
        self.rdom.get_mut(old_node_id).unwrap().remove();
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let old_node_id = self.state.load_child(self.rdom, path);
        for new in new_nodes {
            let mut node = self.rdom.get_mut(new).unwrap();
            node.insert_before(old_node_id);
        }
        self.rdom.get_mut(old_node_id).unwrap().remove();
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let old_node_id = self.state.element_to_node_id(id);
        for new in new_nodes.into_iter().rev() {
            let mut node = self.rdom.get_mut(new).unwrap();
            node.insert_after(old_node_id);
        }
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.state.stack.split_off(self.state.stack.len() - m);
        let old_node_id = self.state.element_to_node_id(id);
        for new in new_nodes {
            self.rdom.tree_mut().insert_before(old_node_id, new);
        }
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let node_id = self.state.element_to_node_id(id);
        let mut node = self.rdom.get_mut(node_id).unwrap();
        let mut node_type_mut = node.node_type_mut();
        if let NodeTypeMut::Element(element) = &mut node_type_mut {
            if let AttributeValue::None = &value {
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

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        let node_id = self.state.element_to_node_id(id);
        let mut node = self.rdom.get_mut(node_id).unwrap();
        let node_type_mut = node.node_type_mut();
        if let NodeTypeMut::Text(mut text) = node_type_mut {
            *text.text_mut() = value.to_string();
        }
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        let node_id = self.state.element_to_node_id(id);
        let mut node = self.rdom.get_mut(node_id).unwrap();
        node.add_event_listener(name);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        let node_id = self.state.element_to_node_id(id);
        let mut node = self.rdom.get_mut(node_id).unwrap();
        node.remove_event_listener(name);
    }

    fn remove_node(&mut self, id: ElementId) {
        let node_id = self.state.element_to_node_id(id);
        self.rdom.get_mut(node_id).unwrap().remove();
    }

    fn push_root(&mut self, id: ElementId) {
        let node_id = self.state.element_to_node_id(id);
        self.state.stack.push(node_id);
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
