use crate::{Template, TemplatePath, VNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DynamicNodeSlot<'a> {
    template: &'a Template,
    index: usize,
    path: TemplatePath,
}

impl<'a> DynamicNodeSlot<'a> {
    pub(super) fn new(template: &'a Template, index: usize, path: TemplatePath) -> Self {
        Self {
            template,
            index,
            path,
        }
    }

    pub(super) fn index(self) -> usize {
        self.index
    }

    pub(super) fn root_index(self) -> usize {
        self.path.segment(0) as usize
    }

    pub(super) fn is_root_level(self) -> bool {
        self.path.is_root_level_slot()
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.path.split_slot().0
    }

    pub(super) fn child_index(self) -> usize {
        self.path.split_slot().1
    }

    pub(super) fn is_inside_static(self, path: TemplatePath) -> bool {
        self.path.slot_is_inside_static(path)
    }

    pub(super) fn root_slot(self) -> Self {
        Self {
            template: self.template,
            index: self.index,
            path: TemplatePath::root(self.root_index()).with_appends(self.path.appends()),
        }
    }

    pub(super) fn placement(self) -> SlotPlacement {
        let parent_path = self.parent_path();
        let child_index = self.child_index();

        SlotPlacement {
            parent_path,
            static_insertion_index: child_index,
            appends: self.path.appends(),
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

#[derive(Clone, Debug)]
pub(super) struct DynamicAttrGroup<'a> {
    template: &'a Template,
    path: TemplatePath,
    start: usize,
    end: usize,
}

impl<'a> DynamicAttrGroup<'a> {
    fn new(template: &'a Template, path: TemplatePath, start: usize, end: usize) -> Self {
        Self {
            template,
            path,
            start,
            end,
        }
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.start..self.end
    }

    pub(super) fn path(&self) -> TemplatePath {
        self.path
    }

    pub(super) fn is_root_level(&self) -> bool {
        self.path.is_root_level_slot()
    }

    pub(super) fn is_descendant_of_static(&self, path: TemplatePath) -> bool {
        self.path.is_descendant_of_static(path)
    }

    pub(super) fn first_id(&self) -> Option<usize> {
        self.ids().next()
    }

    pub(super) fn static_attr_value_for_key(
        &self,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let element_op = self.template.static_node_op_at_path(self.path)?;
        self.template.static_attr_value_for_key(element_op, key)
    }
}

pub(super) fn dynamic_node_slots(
    vnode: &VNode,
) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
    vnode
        .template
        .dynamics()
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, _)| vnode.dynamic_values[*index].as_node().is_some())
        .map(|(index, path)| DynamicNodeSlot::new(&vnode.template, index, path))
}

pub(super) fn dynamic_node_slot(vnode: &VNode, index: usize) -> Option<DynamicNodeSlot<'_>> {
    dynamic_node_slots(vnode).find(|slot| slot.index() == index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TemplateRoot<'a> {
    Static { root_idx: usize, op: usize },
    Dynamic { slot: DynamicNodeSlot<'a> },
}

pub(super) fn template_roots(vnode: &VNode) -> TemplateRoots<'_> {
    TemplateRoots {
        vnode,
        op: 0,
        root_idx: 0,
        dynamic_idx: 0,
    }
}

pub(super) struct TemplateRoots<'a> {
    vnode: &'a VNode,
    op: usize,
    root_idx: usize,
    dynamic_idx: usize,
}

impl<'a> TemplateRoots<'a> {
    fn next_dynamic_root(&mut self, root_idx: usize) -> Option<DynamicNodeSlot<'a>> {
        let template = &self.vnode.template;
        while self.dynamic_idx < template.dynamics().len() {
            let idx = self.dynamic_idx;
            self.dynamic_idx += 1;

            if self.vnode.dynamic_values[idx].as_node().is_none() {
                continue;
            }

            let path = template.dynamic_path(idx);
            if path.is_root_slot(root_idx) {
                return Some(DynamicNodeSlot::new(template, idx, path));
            }
        }
        None
    }
}

impl<'a> Iterator for TemplateRoots<'a> {
    type Item = TemplateRoot<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let template = &self.vnode.template;
        while self.op < template.ops().len() {
            let op = self.op;
            self.op = template.next_sibling_op(op);

            if template.is_static_node_op(op) {
                let root_idx = self.root_idx;
                self.root_idx += 1;
                return Some(TemplateRoot::Static { root_idx, op });
            }

            if template.is_dynamic_node_marker(op) {
                let root_idx = self.root_idx;
                self.root_idx += 1;
                if let Some(slot) = self.next_dynamic_root(root_idx) {
                    return Some(TemplateRoot::Dynamic { slot });
                }
            }
        }
        None
    }
}

pub(super) fn for_each_dynamic_attr_group<'a>(
    vnode: &'a VNode,
    mut visit: impl FnMut(DynamicAttrGroup<'a>),
) {
    let mut current = None;

    for (idx, path) in vnode.template.dynamics().iter().copied().enumerate() {
        if vnode.dynamic_values[idx].as_attrs().is_none() {
            continue;
        }

        match current {
            Some((current_path, start, _)) if current_path == path => {
                current = Some((current_path, start, idx + 1));
            }
            Some((current_path, start, end)) => {
                visit(DynamicAttrGroup::new(
                    &vnode.template,
                    current_path,
                    start,
                    end,
                ));
                current = Some((path, idx, idx + 1));
            }
            None => current = Some((path, idx, idx + 1)),
        }
    }

    if let Some((path, start, end)) = current {
        visit(DynamicAttrGroup::new(&vnode.template, path, start, end));
    }
}
