use crate::{Template, TemplatePath, VNode, template::TemplateAnchor};

/// One dynamic node value (`index`) viewed over its owning [`TemplateAnchor`].
///
/// An anchor can cover several adjacent node values at the same insertion position (e.g. `{a}{b}`);
/// the diff processes each value separately, so this picks out one `index` from `anchor.values()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DynamicNodeSlot<'a> {
    anchor: &'a TemplateAnchor,
    index: usize,
}

impl<'a> DynamicNodeSlot<'a> {
    pub(super) fn new(_template: &'a Template, anchor: &'a TemplateAnchor, index: usize) -> Self {
        Self { anchor, index }
    }

    fn path(self) -> TemplatePath {
        self.anchor.path()
    }

    pub(super) fn index(self) -> usize {
        self.index
    }

    pub(super) fn root_index(self) -> usize {
        self.path().segment(0) as usize
    }

    pub(super) fn is_root_level(self) -> bool {
        self.path().is_root_level_slot()
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.path().split_slot().0
    }

    pub(super) fn child_index(self) -> usize {
        self.path().split_slot().1
    }

    pub(super) fn placement(self) -> SlotPlacement {
        let path = self.path();
        let (parent_path, child_index) = path.split_slot();

        SlotPlacement {
            parent_path,
            static_insertion_index: child_index,
            appends: path.appends(),
        }
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.parent_path() == other.parent_path() && self.child_index() == other.child_index()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SlotPlacement {
    pub(super) parent_path: TemplatePath,
    pub(super) static_insertion_index: usize,
    pub(super) appends: bool,
}

/// A group of dynamic attribute values that all attach to one static element, viewed directly over
/// its [`TemplateAnchor`].
#[derive(Clone, Copy, Debug)]
pub(super) struct DynamicAttrGroup<'a> {
    template: &'a Template,
    anchor: &'a TemplateAnchor,
}

impl<'a> DynamicAttrGroup<'a> {
    pub(super) fn new(template: &'a Template, anchor: &'a TemplateAnchor) -> Self {
        Self { template, anchor }
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.anchor.values()
    }

    pub(super) fn path(&self) -> TemplatePath {
        self.anchor.path()
    }

    pub(super) fn is_root_level(&self) -> bool {
        self.anchor.path().is_root_level_slot()
    }

    pub(super) fn first_id(&self) -> usize {
        self.anchor.value_start()
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
        let is_node = vnode.dynamic_values[anchor.value_start()]
            .as_node()
            .is_some();
        let values = if is_node {
            anchor.values()
        } else {
            anchor.value_start()..anchor.value_start()
        };
        values.map(move |index| DynamicNodeSlot::new(template, anchor, index))
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
        if vnode.dynamic_values[anchor.value_start()]
            .as_attrs()
            .is_some()
        {
            visit(DynamicAttrGroup::new(&vnode.template, anchor));
        }
    }
}
