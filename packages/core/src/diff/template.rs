use crate::{Template, VNode};
use dioxus_core_template::{TemplateAnchor, TemplatePath, TemplateSlotTarget};

/// One dynamic node value (`index`) viewed over its owning [`TemplateAnchor`].
///
/// An anchor can cover several adjacent node values at the same insertion position (e.g. `{a}{b}`);
/// the diff processes each value separately, so this picks out one `index` from `anchor.values()`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct DynamicNodeSlot<'a> {
    anchor: &'a TemplateAnchor,
    anchor_index: usize,
    root_index: usize,
    index: usize,
}

impl<'a> DynamicNodeSlot<'a> {
    fn new(
        anchor: &'a TemplateAnchor,
        anchor_index: usize,
        root_index: usize,
        index: usize,
    ) -> Self {
        Self {
            anchor,
            anchor_index,
            root_index,
            index,
        }
    }

    fn slot_target(self) -> TemplateSlotTarget {
        self.anchor.slot_target()
    }

    pub(super) fn index(self) -> usize {
        self.index
    }

    pub(super) fn anchor_index(self) -> usize {
        self.anchor_index
    }

    pub(super) fn appends(self) -> bool {
        matches!(self.slot_target(), TemplateSlotTarget::AppendChildren(_))
    }

    pub(super) fn root_index(self) -> usize {
        self.root_index
    }

    /// Return true when this dynamic node is inserted at the vnode root level, with no enclosing
    /// static element.
    pub(super) fn is_root_level(self) -> bool {
        match self.slot_target() {
            TemplateSlotTarget::BeforeStatic(path) => path.is_root(),
            TemplateSlotTarget::AppendChildren(path) => path.is_empty(),
        }
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.anchor.static_path()
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.slot_target() == other.slot_target()
    }
}

/// A group of dynamic attribute values that all attach to one static element, viewed directly over
/// its [`TemplateAnchor`].
#[derive(Clone)]
pub(super) struct DynamicAttrGroup<'a> {
    template: &'a Template,
    dynamic_values: &'a [crate::DynamicValue],
    anchor: &'a TemplateAnchor,
    anchor_index: usize,
}

impl<'a> DynamicAttrGroup<'a> {
    pub(super) fn new(vnode: &'a VNode, anchor: &'a TemplateAnchor, anchor_index: usize) -> Self {
        Self {
            template: &vnode.template,
            dynamic_values: &vnode.dynamic_values,
            anchor,
            anchor_index,
        }
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.anchor
            .values()
            .filter(|&idx| self.dynamic_values[idx].as_attrs().is_some())
    }

    pub(super) fn anchor_index(&self) -> usize {
        self.anchor_index
    }

    pub(super) fn static_attr_value_for_key(
        &self,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let element_op = self
            .anchor
            .parent_element_op_index()
            .expect("bad attr anchor");
        self.template.static_attr_value_for_key(element_op, key)
    }
}

pub(super) fn dynamic_node_slots(
    vnode: &VNode,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
    dynamic_node_slots_for_anchors(vnode, vnode.template.anchors().iter().enumerate())
}

pub(super) fn dynamic_node_slots_in_document_order(
    vnode: &VNode,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
    dynamic_node_slots_for_anchors(
        vnode,
        anchors_with_indices_in_document_order(&vnode.template),
    )
}

fn dynamic_node_slots_for_anchors<'a, I>(
    vnode: &'a VNode,
    anchors: I,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'a>> + 'a
where
    I: DoubleEndedIterator<Item = (usize, &'static TemplateAnchor)> + 'a,
{
    let template = &vnode.template;
    anchors.flat_map(move |(anchor_index, anchor)| {
        let root_index = anchor_root_index(template, anchor);
        anchor
            .values()
            .filter(|&index| vnode.dynamic_values[index].as_node().is_some())
            .map(move |index| DynamicNodeSlot::new(anchor, anchor_index, root_index, index))
    })
}

pub(super) fn dynamic_node_slot(vnode: &VNode, index: usize) -> Option<DynamicNodeSlot<'_>> {
    dynamic_node_slots(vnode).find(|slot| slot.index() == index)
}

pub(super) fn for_each_dynamic_attr_group<'a>(
    vnode: &'a VNode,
    mut visit: impl FnMut(DynamicAttrGroup<'a>),
) {
    for (anchor_index, anchor) in vnode.template.anchors().iter().enumerate() {
        let group = DynamicAttrGroup::new(vnode, anchor, anchor_index);
        // Anchors that carry only dynamic nodes (e.g. a root-level node slot)
        // decorate no static element, so they are not attribute groups.
        if group.ids().next().is_some() {
            visit(group);
        }
    }
}

fn anchors_with_indices_in_document_order(
    template: &Template,
) -> impl DoubleEndedIterator<Item = (usize, &'static TemplateAnchor)> + '_ {
    let value_count = template
        .anchors()
        .iter()
        .map(|anchor| anchor.values().end)
        .max()
        .unwrap_or(0);

    (0..value_count).filter_map(move |idx| {
        template
            .anchors()
            .iter()
            .enumerate()
            .find(|(_, anchor)| anchor.values().start == idx)
    })
}

fn anchor_root_index(template: &Template, anchor: &TemplateAnchor) -> usize {
    if anchor_is_root_level(anchor) {
        for (root_idx, _, dynamic_anchor) in template.root_slots() {
            if dynamic_anchor.is_some_and(|candidate| *candidate == *anchor) {
                return root_idx;
            }
        }
        panic!("bad root slot");
    }

    let static_root_idx = match anchor.slot_target() {
        TemplateSlotTarget::BeforeStatic(path) => Some(path.segment(0) as usize),
        TemplateSlotTarget::AppendChildren(path) => {
            (!path.is_empty()).then(|| path.segment(0) as usize)
        }
    }
    .expect("bad slot root");

    template
        .materialization_root_for_static(static_root_idx)
        .expect("bad slot root")
}

fn anchor_is_root_level(anchor: &TemplateAnchor) -> bool {
    match anchor.slot_target() {
        TemplateSlotTarget::BeforeStatic(path) => path.is_root(),
        TemplateSlotTarget::AppendChildren(path) => path.is_empty(),
    }
}
