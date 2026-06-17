use crate::{
    Template, TemplatePath, VNode,
    template::{TemplateAnchor, TemplateSlotPath, TemplateSlotTarget},
};

/// One dynamic node value (`index`) viewed over its owning [`TemplateAnchor`].
///
/// An anchor can cover several adjacent node values at the same insertion position (e.g. `{a}{b}`);
/// the diff processes each value separately, so this picks out one `index` from `anchor.values()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DynamicNodeSlot<'a> {
    template: &'a Template,
    anchor: &'a TemplateAnchor,
    index: usize,
}

impl<'a> DynamicNodeSlot<'a> {
    pub(super) fn new(template: &'a Template, anchor: &'a TemplateAnchor, index: usize) -> Self {
        Self {
            template,
            anchor,
            index,
        }
    }

    pub(super) fn slot_path(self) -> TemplateSlotPath {
        self.anchor.slot_path()
    }

    pub(super) fn index(self) -> usize {
        self.index
    }

    pub(super) fn root_index(self) -> usize {
        if self.is_root_level() {
            for (root_idx, _, dynamic_anchor) in self.template.root_slots() {
                if dynamic_anchor.is_some_and(|anchor| *anchor == *self.anchor) {
                    return root_idx;
                }
            }
            panic!("root-level dynamic slot must appear in template root slots");
        }

        let static_root_idx = self
            .slot_path()
            .root_index()
            .expect("non-root dynamic slot must target a static root");
        let mut current_static_root = 0;
        for (root_idx, static_op, _) in self.template.root_slots() {
            if static_op.is_none() {
                continue;
            }
            if current_static_root == static_root_idx {
                return root_idx;
            }
            current_static_root += 1;
        }
        panic!("non-root dynamic slot must belong to a static template root");
    }

    pub(super) fn is_root_level(self) -> bool {
        self.slot_path().is_root_level()
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.slot_path().static_parent()
    }

    pub(super) fn placement(self) -> SlotPlacement {
        let target = self.slot_path();
        match target.target() {
            TemplateSlotTarget::BeforeStatic(path) => {
                let (parent_path, static_insertion_index) = split_static_path(path);
                SlotPlacement {
                    parent_path,
                    static_insertion_index,
                    appends: false,
                }
            }
            TemplateSlotTarget::AppendChildren(parent_path) => SlotPlacement {
                parent_path,
                static_insertion_index: 0,
                appends: true,
            },
        }
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.slot_path() == other.slot_path()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SlotPlacement {
    pub(super) parent_path: TemplatePath,
    pub(super) static_insertion_index: usize,
    pub(super) appends: bool,
}

fn split_static_path(path: TemplatePath) -> (TemplatePath, usize) {
    let mut parent = path.bits();
    let mut insertion_index = 0usize;
    while parent != 0 && parent & 1 == 0 {
        insertion_index += 1;
        parent >>= 1;
    }
    if parent != 0 {
        parent >>= 1;
    }
    (TemplatePath::from_bits(parent), insertion_index)
}

/// A group of dynamic attribute values that all attach to one static element, viewed directly over
/// its [`TemplateAnchor`].
#[derive(Clone, Debug)]
pub(super) struct DynamicAttrGroup<'a> {
    template: &'a Template,
    dynamic_values: &'a [crate::DynamicValue],
    anchor: &'a TemplateAnchor,
}

impl<'a> DynamicAttrGroup<'a> {
    pub(super) fn new(vnode: &'a VNode, anchor: &'a TemplateAnchor) -> Self {
        Self {
            template: &vnode.template,
            dynamic_values: &vnode.dynamic_values,
            anchor,
        }
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.anchor
            .values()
            .filter(|&idx| self.dynamic_values[idx].as_attrs().is_some())
    }

    pub(super) fn has_attrs(&self) -> bool {
        self.ids().next().is_some()
    }

    pub(super) fn static_path(&self) -> TemplatePath {
        self.anchor.static_path()
    }

    pub(super) fn is_root_level(&self) -> bool {
        self.anchor.static_path().len() == 1
    }

    pub(super) fn first_id(&self) -> usize {
        self.ids()
            .next()
            .expect("dynamic attribute group must contain at least one dynamic attribute")
    }

    pub(super) fn static_attr_value_for_key(
        &self,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let element_op = self
            .anchor
            .element_op()
            .expect("a dynamic attribute anchor always has an enclosing element");
        self.template.static_attr_value_for_key(element_op, key)
    }
}

pub(super) fn dynamic_node_slots(
    vnode: &VNode,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
    dynamic_node_slots_for_anchors(vnode, vnode.template.anchors().iter())
}

pub(super) fn dynamic_node_slots_in_document_order(
    vnode: &VNode,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
    dynamic_node_slots_for_anchors(vnode, vnode.template.anchors_in_document_order())
}

fn dynamic_node_slots_for_anchors<'a, I>(
    vnode: &'a VNode,
    anchors: I,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'a>> + 'a
where
    I: DoubleEndedIterator<Item = &'static TemplateAnchor> + 'a,
{
    let template = &vnode.template;
    anchors.flat_map(move |anchor| {
        anchor
            .values()
            .filter(|&index| vnode.dynamic_values[index].as_node().is_some())
            .map(move |index| DynamicNodeSlot::new(template, anchor, index))
    })
}

pub(super) fn dynamic_node_slot(vnode: &VNode, index: usize) -> Option<DynamicNodeSlot<'_>> {
    dynamic_node_slots(vnode).find(|slot| slot.index() == index)
}

pub(super) fn for_each_dynamic_attr_group<'a>(
    vnode: &'a VNode,
    mut visit: impl FnMut(DynamicAttrGroup<'a>),
) {
    for anchor in vnode.template.anchors() {
        let group = DynamicAttrGroup::new(vnode, anchor);
        if group.has_attrs() {
            visit(group);
        }
    }
}
