use crate::{state::State, tree::NodeId, RealNodeId};
use dioxus_core::ElementId;
use rustc_hash::{FxHashMap, FxHashSet};

/// The node is stored client side and stores only basic data about the node.
#[derive(Debug, Clone)]
pub struct Node<S: State> {
    /// The transformed state of the node.
    pub state: S,
    /// The raw data for the node
    pub node_data: NodeData,
}

#[derive(Debug, Clone)]
pub struct NodeData {
    /// The id of the node in the vdom.
    pub element_id: Option<ElementId>,
    /// Additional inforation specific to the node type
    pub node_type: NodeType,
    /// The number of parents before the root node. The root node has height 1.
    pub height: u16,
}

/// A type of node with data specific to the node type. The types are a subset of the [VNode] types.
#[derive(Debug, Clone)]
pub enum NodeType {
    Text {
        text: String,
    },
    Element {
        tag: String,
        namespace: Option<&'static str>,
        attributes: FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue>,
        listeners: FxHashSet<String>,
    },
    Placeholder,
}

impl<S: State> Node<S> {
    pub(crate) fn new(node_type: NodeType) -> Self {
        Node {
            state: S::default(),
            node_data: NodeData {
                element_id: None,
                node_type,
                height: 0,
            },
        }
    }

    /// link a child node
    fn add_child(&mut self, child: RealNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.push(child);
        }
    }

    /// remove a child node
    fn remove_child(&mut self, child: RealNodeId) {
        if let NodeType::Element { children, .. } = &mut self.node_data.node_type {
            children.retain(|c| c != &child);
        }
    }

    /// link the parent node
    fn set_parent(&mut self, parent: RealNodeId) {
        self.node_data.parent = Some(parent);
    }

    /// get the mounted id of the node
    pub fn mounted_id(&self) -> Option<ElementId> {
        self.node_data.element_id
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct OwnedAttributeDiscription {
    pub name: String,
    pub namespace: Option<String>,
    pub volatile: bool,
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Copy, Debug)]
pub struct OwnedAttributeView<'a> {
    /// The discription of the attribute.
    pub attribute: &'a OwnedAttributeDiscription,

    /// The value of the attribute.
    pub value: &'a OwnedAttributeValue,
}

#[derive(Clone, Debug)]
pub enum OwnedAttributeValue {
    Text(String),
    Float(f32),
    Int(i32),
    Bool(bool),
    None,
}
