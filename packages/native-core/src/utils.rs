use crate::{node::NodeType, real_dom::RealDom, state::State, tree::TreeView, NodeId, RealNodeId};
use dioxus_core::{Mutation, Mutations};

pub enum ElementProduced {
    /// The iterator produced an element by progressing to the next node in a depth first order.
    Progressed(RealNodeId),
    /// The iterator reached the end of the tree and looped back to the root
    Looped(RealNodeId),
}
impl ElementProduced {
    pub fn id(&self) -> RealNodeId {
        match self {
            ElementProduced::Progressed(id) => *id,
            ElementProduced::Looped(id) => *id,
        }
    }
}

#[derive(Debug)]
enum NodePosition {
    AtNode,
    InChild(usize),
}

impl NodePosition {
    fn map(&self, mut f: impl FnMut(usize) -> usize) -> Self {
        match self {
            Self::AtNode => Self::AtNode,
            Self::InChild(i) => Self::InChild(f(*i)),
        }
    }

    fn get_or_insert(&mut self, child_idx: usize) -> usize {
        match self {
            Self::AtNode => {
                *self = Self::InChild(child_idx);
                child_idx
            }
            Self::InChild(i) => *i,
        }
    }
}

/// Focus systems need a iterator that can persist through changes in the [dioxus_core::VirtualDom].
/// This iterator traverses the tree depth first.
/// Iterate through it with [PersistantElementIter::next] [PersistantElementIter::prev], and update it with [PersistantElementIter::prune] (with data from [`dioxus_core::prelude::VirtualDom::work_with_deadline`]).
/// The iterator loops around when it reaches the end or the beginning.
pub struct PersistantElementIter {
    // stack of elements and fragments
    stack: smallvec::SmallVec<[(RealNodeId, NodePosition); 5]>,
}

impl Default for PersistantElementIter {
    fn default() -> Self {
        PersistantElementIter {
            stack: smallvec::smallvec![(NodeId(0), NodePosition::AtNode)],
        }
    }
}

impl PersistantElementIter {
    pub fn new() -> Self {
        Self::default()
    }

    /// remove stale element refreneces
    /// returns true if the focused element is removed
    pub fn prune<S: State>(&mut self, mutations: &Mutations, rdom: &RealDom<S>) -> bool {
        let mut changed = false;
        let ids_removed: Vec<_> = mutations
            .edits
            .iter()
            .filter_map(|m| {
                // nodes within templates will never be removedns
                if let Mutation::Remove { id } = m {
                    Some(rdom.element_to_node_id(*id))
                } else {
                    None
                }
            })
            .collect();
        // if any element is removed in the chain, remove it and its children from the stack
        if let Some(r) = self
            .stack
            .iter()
            .position(|(el_id, _)| ids_removed.iter().any(|id| el_id == id))
        {
            self.stack.truncate(r);
            changed = true;
        }
        // if a child is removed or inserted before or at the current element, update the child index
        for (el_id, child_idx) in self.stack.iter_mut() {
            if let NodePosition::InChild(child_idx) = child_idx {
                if let Some(children) = &rdom.tree.children_ids(*el_id) {
                    for m in &mutations.edits {
                        match m {
                            Mutation::Remove { id } => {
                                let id = rdom.element_to_node_id(*id);
                                if children.iter().take(*child_idx + 1).any(|c| *c == id) {
                                    *child_idx -= 1;
                                }
                            }
                            Mutation::InsertBefore { id, m } => {
                                let id = rdom.element_to_node_id(*id);
                                if children.iter().take(*child_idx + 1).any(|c| *c == id) {
                                    *child_idx += *m;
                                }
                            }
                            Mutation::InsertAfter { id, m } => {
                                let id = rdom.element_to_node_id(*id);
                                if children.iter().take(*child_idx).any(|c| *c == id) {
                                    *child_idx += *m;
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
        changed
    }

    /// get the next element
    pub fn next<S: State>(&mut self, rdom: &RealDom<S>) -> ElementProduced {
        if self.stack.is_empty() {
            let id = NodeId(0);
            let new = (id, NodePosition::AtNode);
            self.stack.push(new);
            ElementProduced::Looped(id)
        } else {
            let (last, o_child_idx) = self.stack.last_mut().unwrap();
            let node = &rdom[*last];
            match &node.node_data.node_type {
                NodeType::Element { .. } => {
                    let children = rdom.tree.children_ids(*last).unwrap();
                    *o_child_idx = o_child_idx.map(|i| i + 1);
                    // if we have children, go to the next child
                    let child_idx = o_child_idx.get_or_insert(0);
                    if child_idx >= children.len() {
                        self.pop();
                        self.next(rdom)
                    } else {
                        let id = children[child_idx];
                        if let NodeType::Element { .. } = &rdom[id].node_data.node_type {
                            self.stack.push((id, NodePosition::AtNode));
                        }
                        ElementProduced::Progressed(id)
                    }
                }

                NodeType::Text { .. } | NodeType::Placeholder { .. } => {
                    // we are at a leaf, so we are done
                    ElementProduced::Progressed(self.pop())
                }
            }
        }
    }

    /// get the previous element
    pub fn prev<S: State>(&mut self, rdom: &RealDom<S>) -> ElementProduced {
        // recursively add the last child element to the stack
        fn push_back<S: State>(
            stack: &mut smallvec::SmallVec<[(RealNodeId, NodePosition); 5]>,
            new_node: RealNodeId,
            rdom: &RealDom<S>,
        ) -> RealNodeId {
            match &rdom[new_node].node_data.node_type {
                NodeType::Element { .. } => {
                    let children = rdom.tree.children_ids(new_node).unwrap();
                    if children.is_empty() {
                        new_node
                    } else {
                        stack.push((new_node, NodePosition::InChild(children.len() - 1)));
                        push_back(stack, *children.last().unwrap(), rdom)
                    }
                }
                _ => new_node,
            }
        }
        if self.stack.is_empty() {
            let new_node = NodeId(0);
            ElementProduced::Looped(push_back(&mut self.stack, new_node, rdom))
        } else {
            let (last, o_child_idx) = self.stack.last_mut().unwrap();
            let node = &rdom[*last];
            match &node.node_data.node_type {
                NodeType::Element { .. } => {
                    let children = rdom.tree.children_ids(*last).unwrap();
                    // if we have children, go to the next child
                    if let NodePosition::InChild(0) = o_child_idx {
                        ElementProduced::Progressed(self.pop())
                    } else {
                        *o_child_idx = o_child_idx.map(|i| i - 1);
                        if let NodePosition::InChild(child_idx) = o_child_idx {
                            if *child_idx >= children.len() || children.is_empty() {
                                self.pop();
                                self.prev(rdom)
                            } else {
                                let new_node = children[*child_idx];
                                ElementProduced::Progressed(push_back(
                                    &mut self.stack,
                                    new_node,
                                    rdom,
                                ))
                            }
                        } else {
                            self.pop();
                            self.prev(rdom)
                        }
                    }
                }

                NodeType::Text { .. } | NodeType::Placeholder { .. } => {
                    // we are at a leaf, so we are done
                    ElementProduced::Progressed(self.pop())
                }
            }
        }
    }

    fn pop(&mut self) -> RealNodeId {
        self.stack.pop().unwrap().0
    }
}
