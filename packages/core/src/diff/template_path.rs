use crate::{TemplateCursor, TemplateNode, WriteMutations};

pub(super) fn split_slot_cursor(cursor_tail: &'static [u8]) -> (&'static [u8], u8) {
    let (slot_index, parent_cursor) = cursor_tail
        .split_last()
        .expect("slot cursors must point at a dynamic child");
    (parent_cursor, *slot_index)
}

pub(super) fn slot_appends(
    root: &'static TemplateNode,
    parent_cursor: &'static [u8],
    slot_index: u8,
) -> bool {
    let parent = root
        .node_at_child_cursor(parent_cursor)
        .expect("slot parent must exist in the template");
    let TemplateNode::Element { children, .. } = parent else {
        unreachable!("slot cursors only point into template elements")
    };
    slot_index as usize >= children.len()
}

pub(super) fn push_static_cursor(
    to: &mut dyn WriteMutations,
    mut node: &'static TemplateNode,
    cursor: &[u8],
) {
    for step in cursor {
        let index = *step as usize;
        to.child(index);
        node = node.element_child(index);
    }
}

pub(super) fn cursor_starts_with(cursor: TemplateCursor, root_idx: u8) -> bool {
    cursor.as_slice().first() == Some(&root_idx)
}
