use std::ops::{Index, IndexMut};

use web_sys::Node;

pub struct NodeSlab {
    nodes: Vec<Option<Node>>,
}

impl NodeSlab {
    pub fn new(capacity: usize) -> NodeSlab {
        let mut nodes = Vec::with_capacity(capacity);
        for x in 0..5 {
            nodes.push(None);
        }

        NodeSlab { nodes }
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
        if index >= self.nodes.capacity() * 3 {
            panic!("Trying to mutate an element way too far out of bounds");
        }

        if index + 1 > self.nodes.len() {
            self.nodes.resize_with(index + 1, || None);
        }
        &mut self.nodes[index]
    }
}
