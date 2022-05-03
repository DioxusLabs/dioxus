use crate::{
    real_dom::{NodeType, RealDom},
    state::State,
};
use dioxus_core::{DomEdit, ElementId, Mutations};

pub enum ElementProduced {
    Progressed(ElementId),
    Looped(ElementId),
}
impl ElementProduced {
    pub fn id(&self) -> ElementId {
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

/// The focus system needs a iterator that can persist through changes in the [VirtualDom].
/// Iterate through it with [ElementIter::next] [ElementIter::prev], and update it with [ElementIter::update] (with data from [`VirtualDom::work_with_deadline`]).
/// The iterator loops around when it reaches the end or the beginning.
pub struct PersistantElementIter {
    // stack of elements and fragments
    stack: smallvec::SmallVec<[(ElementId, NodePosition); 5]>,
}

impl Default for PersistantElementIter {
    fn default() -> Self {
        PersistantElementIter {
            stack: smallvec::smallvec![(ElementId(0), NodePosition::AtNode)],
        }
    }
}

impl PersistantElementIter {
    pub fn new() -> Self {
        Self::default()
    }

    /// remove stale element refreneces
    pub fn prune<S: State>(&mut self, mutations: &Mutations, rdom: &RealDom<S>) {
        let ids_removed: Vec<_> = mutations
            .edits
            .iter()
            .filter_map(|e| {
                if let DomEdit::Remove { root } = e {
                    Some(*root)
                } else {
                    None
                }
            })
            .collect();
        if let Some(r) = self
            .stack
            .iter()
            .position(|(el_id, _)| ids_removed.iter().any(|id| el_id.as_u64() == *id))
        {
            self.stack.truncate(r);
        }
        // if a child is removed or inserted before or at the current element, update the child index
        for (el_id, child_idx) in self.stack.iter_mut() {
            if let NodePosition::InChild(child_idx) = child_idx {
                if let NodeType::Element { children, .. } = &rdom[*el_id].node_type {
                    for m in &mutations.edits {
                        match m {
                            DomEdit::Remove { root } => {
                                if children
                                    .iter()
                                    .take(*child_idx + 1)
                                    .any(|c| c.as_u64() == *root)
                                {
                                    *child_idx -= 1;
                                }
                            }
                            DomEdit::InsertBefore { root, n } => {
                                if children
                                    .iter()
                                    .take(*child_idx + 1)
                                    .any(|c| c.as_u64() == *root)
                                {
                                    *child_idx += *n as usize;
                                }
                            }
                            DomEdit::InsertAfter { root, n } => {
                                if children
                                    .iter()
                                    .take(*child_idx)
                                    .any(|c| c.as_u64() == *root)
                                {
                                    *child_idx += *n as usize;
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }

    /// get the next element
    pub fn next<S: State>(&mut self, rdom: &RealDom<S>) -> ElementProduced {
        if self.stack.is_empty() {
            let id = ElementId(0);
            let new = (id, NodePosition::AtNode);
            self.stack.push(new);
            ElementProduced::Looped(id)
        } else {
            let (last, o_child_idx) = self.stack.last_mut().unwrap();
            let node = &rdom[*last];
            match &node.node_type {
                NodeType::Element { children, .. } => {
                    *o_child_idx = o_child_idx.map(|i| i + 1);
                    // if we have children, go to the next child
                    let child_idx = o_child_idx.get_or_insert(0);
                    if child_idx >= children.len() {
                        self.pop();
                        self.next(rdom)
                    } else {
                        let id = children[child_idx];
                        if let NodeType::Element { .. } = &rdom[id].node_type {
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
        fn push_back<S: State>(
            stack: &mut smallvec::SmallVec<[(ElementId, NodePosition); 5]>,
            new_node: ElementId,
            rdom: &RealDom<S>,
        ) -> ElementId {
            match &rdom[new_node].node_type {
                NodeType::Element { children, .. } => {
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
            let new_node = ElementId(0);
            ElementProduced::Looped(push_back(&mut self.stack, new_node, rdom))
        } else {
            let (last, o_child_idx) = self.stack.last_mut().unwrap();
            let node = &rdom[*last];
            match &node.node_type {
                NodeType::Element { children, .. } => {
                    // if we have children, go to the next child
                    if let NodePosition::InChild(0) = o_child_idx {
                        ElementProduced::Progressed(self.pop())
                    } else {
                        *o_child_idx = o_child_idx.map(|i| i - 1);
                        if let NodePosition::InChild(child_idx) = o_child_idx {
                            if *child_idx >= children.len() || children.len() == 0 {
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

    fn pop(&mut self) -> ElementId {
        self.stack.pop().unwrap().0
    }
}
