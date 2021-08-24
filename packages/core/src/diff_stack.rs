use crate::innerlude::*;
use smallvec::{smallvec, SmallVec};

/// The stack instructions we use to diff and create new nodes.
#[derive(Debug)]
pub enum DiffInstruction<'a> {
    DiffNode {
        old: &'a VNode<'a>,
        new: &'a VNode<'a>,
    },

    DiffChildren {
        old: &'a [VNode<'a>],
        new: &'a [VNode<'a>],
    },

    Create {
        node: &'a VNode<'a>,
    },

    /// pushes the node elements onto the stack for use in mount
    PrepareMoveNode {
        node: &'a VNode<'a>,
    },

    Mount {
        and: MountType<'a>,
    },

    PopElement,

    PopScope,
}

#[derive(Debug, Clone, Copy)]
pub enum MountType<'a> {
    Absorb,
    Append,
    Replace { old: &'a VNode<'a> },
    ReplaceByElementId { el: ElementId },
    InsertAfter { other_node: &'a VNode<'a> },
    InsertBefore { other_node: &'a VNode<'a> },
}

pub struct DiffStack<'bump> {
    instructions: Vec<DiffInstruction<'bump>>,
    nodes_created_stack: SmallVec<[usize; 10]>,
    pub scope_stack: SmallVec<[ScopeId; 5]>,
}

impl<'bump> DiffStack<'bump> {
    pub fn new(cur_scope: ScopeId) -> Self {
        Self {
            instructions: Vec::with_capacity(1000),
            nodes_created_stack: smallvec![],
            scope_stack: smallvec![cur_scope],
        }
    }

    pub fn pop(&mut self) -> Option<DiffInstruction<'bump>> {
        self.instructions.pop()
    }

    pub fn pop_scope(&mut self) -> Option<ScopeId> {
        self.scope_stack.pop()
    }

    pub fn push(&mut self, instruction: DiffInstruction<'bump>) {
        self.instructions.push(instruction)
    }

    pub fn create_children(&mut self, children: &'bump [VNode<'bump>], and: MountType<'bump>) {
        self.nodes_created_stack.push(0);
        self.instructions.push(DiffInstruction::Mount { and });

        for child in children.into_iter().rev() {
            self.instructions
                .push(DiffInstruction::Create { node: child });
        }
    }

    pub fn push_nodes_created(&mut self, count: usize) {
        self.nodes_created_stack.push(count);
    }

    pub fn create_node(&mut self, node: &'bump VNode<'bump>, and: MountType<'bump>) {
        self.nodes_created_stack.push(0);
        self.instructions.push(DiffInstruction::Mount { and });
        self.instructions.push(DiffInstruction::Create { node });
    }

    pub fn add_child_count(&mut self, count: usize) {
        *self.nodes_created_stack.last_mut().unwrap() += count;
    }

    pub fn pop_nodes_created(&mut self) -> usize {
        self.nodes_created_stack.pop().unwrap()
    }

    pub fn current_scope(&self) -> Option<ScopeId> {
        self.scope_stack.last().map(|f| f.clone())
    }

    pub fn create_component(&mut self, idx: ScopeId, node: &'bump VNode<'bump>) {
        // Push the new scope onto the stack
        self.scope_stack.push(idx);

        self.instructions.push(DiffInstruction::PopScope);

        // Run the creation algorithm with this scope on the stack
        // ?? I think we treat components as framgnets??
        self.instructions.push(DiffInstruction::Create { node });
    }
}
