use dioxus_core::RendererTemplateId;

use crate::{real_dom::Node, state::State};

#[derive(Debug, Default)]
pub struct NativeTemplate<S: State> {
    pub(crate) nodes: Vec<Option<Node<S>>>,
    pub(crate) roots: Vec<usize>,
}

impl<S: State> NativeTemplate<S> {
    pub fn insert(&mut self, node: Node<S>) {
        let id = node.id.0;
        self.nodes.resize(id, None);
        self.nodes[id] = Some(node);
    }
}

#[derive(Debug)]
pub(crate) enum TemplateRefOrNode<S: State> {
    Ref {
        id: RendererTemplateId,
        overrides: Vec<Box<Node<S>>>,
    },
    Node(Node<S>),
}
