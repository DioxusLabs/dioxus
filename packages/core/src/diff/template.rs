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
        path_starts_with(self.parent_path(), path)
    }

    pub(super) fn root_slot(self) -> Self {
        Self {
            template: self.template,
            index: self.index,
            path: TemplatePath::root(self.root_index()),
        }
    }

    pub(super) fn placement(self) -> SlotPlacement {
        let parent_path = self.parent_path();
        let child_index = self.child_index();
        let static_child_indexes = (!parent_path.is_empty())
            .then(|| self.template.static_prototype_child_indexes(parent_path))
            .flatten();
        let static_insertion_index = if parent_path.is_empty() {
            root_static_insertion_index(self.template, child_index)
        } else {
            self.template
                .static_prototype_insertion_index(parent_path, child_index)
                .unwrap_or(child_index)
        };
        let appends = if parent_path.is_empty() {
            static_insertion_index >= static_root_count(self.template)
        } else {
            slot_appends(self.template, parent_path, child_index)
        };

        SlotPlacement {
            parent_path,
            static_child_indexes,
            static_insertion_index,
            appends,
        }
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.parent_path() == other.parent_path()
            && self.placement().static_insertion_index == other.placement().static_insertion_index
    }
}

#[derive(Clone, Debug)]
pub(super) struct SlotPlacement {
    pub(super) parent_path: TemplatePath,
    pub(super) static_child_indexes: Option<Vec<usize>>,
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
        (self.start..self.end).filter(|idx| {
            self.template.dynamic_is_attr(*idx) && self.template.dynamic_path(*idx) == self.path
        })
    }

    pub(super) fn path(&self) -> TemplatePath {
        self.path
    }

    pub(super) fn is_root_level(&self) -> bool {
        self.path.is_root_level_slot()
    }

    pub(super) fn is_descendant_of_static(&self, path: TemplatePath) -> bool {
        path_starts_with(self.path, path)
    }

    pub(super) fn static_child_indexes(&self) -> Option<Vec<usize>> {
        (!self.path.is_root_level_slot())
            .then(|| self.template.static_prototype_child_indexes(self.path))
            .flatten()
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
        .filter(|(index, _)| vnode.template.dynamic_is_node(*index))
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

            if !template.dynamic_is_node(idx) {
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

    for (idx, path) in vnode.template.attr_paths() {
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

fn root_static_insertion_index(template: &Template, child_index: usize) -> usize {
    (0..child_index)
        .filter(|root| template.root_op_index(*root).is_some())
        .count()
}

fn static_root_count(template: &Template) -> usize {
    (0..template.root_count())
        .filter(|root| template.root_op_index(*root).is_some())
        .count()
}

fn slot_appends(template: &Template, parent_path: TemplatePath, child_index: usize) -> bool {
    let Some(parent_op) = template.static_node_op_at_path(parent_path) else {
        return true;
    };
    let Some(static_slot_index) = template.static_child_insertion_index(parent_op, child_index)
    else {
        return true;
    };
    static_slot_index >= static_child_count(template, parent_op)
}

fn static_child_count(template: &Template, element_op: usize) -> usize {
    let Some(mut cursor) = template.first_child_node_op(element_op) else {
        return 0;
    };
    let Some(end) = template.element_end(element_op) else {
        return 0;
    };

    let mut count = 0;
    while cursor < end {
        if template.is_static_node_op(cursor) {
            count += 1;
        }
        cursor = template.next_sibling_op(cursor);
    }
    count
}

fn path_starts_with(path: TemplatePath, ancestor: TemplatePath) -> bool {
    if ancestor.is_empty() {
        return true;
    }

    if path.len() < ancestor.len() {
        return false;
    }

    (0..ancestor.len()).all(|index| path.segment(index) == ancestor.segment(index))
}
