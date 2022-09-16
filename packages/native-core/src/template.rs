use dioxus_core::{ElementId, GlobalNodeId, RendererTemplateId};

use crate::{real_dom::Node, state::State};

#[derive(Debug, Default)]
pub struct NativeTemplate<S: State> {
    pub(crate) nodes: Vec<Option<Box<Node<S>>>>,
    pub(crate) roots: Vec<usize>,
}

impl<S: State> NativeTemplate<S> {
    pub fn insert(&mut self, node: Node<S>) {
        let id = node.id.0;
        self.nodes.resize(id, None);
        self.nodes[id] = Some(Box::new(node));
    }
}

#[derive(Debug)]
pub(crate) enum TemplateRefOrNode<S: State> {
    Ref {
        id: ElementId,
        nodes: Vec<Option<Box<Node<S>>>>,
        parent: Option<GlobalNodeId>,
    },
    Node(Node<S>),
}

impl<S: State> TemplateRefOrNode<S> {
    pub fn parent(&self) -> Option<GlobalNodeId> {
        match self {
            TemplateRefOrNode::Ref { parent, .. } => *parent,
            TemplateRefOrNode::Node(node) => node.parent,
        }
    }
}
