use std::ops::{Index, IndexMut};

use web_sys::Node;

pub struct NodeSlab {
    nodes: Vec<Option<Node>>,
}

impl NodeSlab {
    pub fn new(capacity: usize) -> NodeSlab {
        NodeSlab {
            nodes: Vec::with_capacity(capacity),
        }
    }

    fn insert_and_extend(&mut self, node: Node, id: usize) {
        if id > self.nodes.len() * 3 {
            panic!("Trying to insert an element way too far out of bounds");
        }

        if id < self.nodes.len() {}
    }
}
impl Index<usize> for NodeSlab {
    type Output = Option<Node>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

impl IndexMut<usize> for NodeSlab {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.nodes.len() * 3 {
            panic!("Trying to mutate an element way too far out of bounds");
        }
        if index > self.nodes.len() {
            self.nodes.resize_with(index, || None);
        }
        &mut self.nodes[index]
    }
}
