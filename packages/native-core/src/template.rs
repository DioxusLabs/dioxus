use dioxus_core::{GlobalNodeId, TemplateNodeId};

use crate::{real_dom::Node, state::State};

#[derive(Debug, Default)]
pub struct NativeTemplate<S: State> {
    pub(crate) nodes: Vec<Option<Box<Node<S>>>>,
    pub(crate) roots: Vec<usize>,
}

impl<S: State> NativeTemplate<S> {
    pub fn insert(&mut self, node: Node<S>) {
        let id = node.node_data.id;
        match id {
            GlobalNodeId::TemplateId {
                template_node_id: TemplateNodeId(id),
                ..
            } => {
                self.nodes.resize(id + 1, None);
                self.nodes[id] = Some(Box::new(node));
            }
            GlobalNodeId::VNodeId(_) => panic!("Cannot insert a VNode into a template"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum TemplateRefOrNode<S: State> {
    Ref {
        nodes: Vec<Option<Box<Node<S>>>>,
        roots: Vec<GlobalNodeId>,
        parent: Option<GlobalNodeId>,
    },
    Node(Node<S>),
}

impl<S: State> TemplateRefOrNode<S> {
    pub fn parent(&self) -> Option<GlobalNodeId> {
        match self {
            TemplateRefOrNode::Ref { parent, .. } => *parent,
            TemplateRefOrNode::Node(node) => node.node_data.parent,
        }
    }
}
