use crate::{AttributeValue, Template, VNode};
use dioxus_core_template::{TemplateAnchor, TemplatePath, TemplateSlotTarget};

/// A rendered child of a [`VNode`] or a static template element.
#[derive(Clone, Copy)]
pub enum VNodeChild<'a> {
    /// A static template element.
    Element(StaticElement<'a>),
    /// A static template text node.
    Text(StaticText<'a>),
    /// One or more dynamic node values at the same insertion position.
    Dynamic(DynamicNodeGroup<'a>),
}

/// A static template element viewed through a specific rendered [`VNode`].
#[derive(Clone, Copy)]
pub struct StaticElement<'a> {
    vnode: &'a VNode,
    op: usize,
    root_position: Option<usize>,
}

impl<'a> StaticElement<'a> {
    pub(crate) fn new(vnode: &'a VNode, op: usize, root_position: Option<usize>) -> Self {
        Self {
            vnode,
            op,
            root_position,
        }
    }

    /// The flat template op for this element.
    pub fn op(self) -> usize {
        self.op
    }

    /// The element tag.
    pub fn tag(self) -> &'static str {
        self.vnode
            .template
            .element_meta_at_op(self.op)
            .expect("static element")
            .0
    }

    /// The element namespace.
    pub fn namespace(self) -> Option<&'static str> {
        self.vnode
            .template
            .element_meta_at_op(self.op)
            .expect("static element")
            .1
    }

    /// The root position when this element is a vnode root.
    pub fn root_position(self) -> Option<usize> {
        self.root_position
    }

    /// Iterate effective attributes for this element.
    pub fn attributes(self) -> ElementAttributes<'a> {
        ElementAttributes::new(self)
    }

    /// Iterate static template attributes for this element.
    pub fn static_attributes(
        self,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + 'a {
        self.vnode.template.static_attrs(self.op)
    }

    /// Iterate rendered children for this element.
    pub fn children(self) -> VNodeChildren<'a> {
        VNodeChildren::element(self)
    }

    /// Return true if this element has any rendered child.
    pub fn has_children(self) -> bool {
        self.children().next().is_some()
    }

    /// Iterate dynamic attribute groups that target this element.
    pub fn dynamic_attributes(self) -> impl Iterator<Item = DynamicAttrGroup<'a>> + 'a {
        self.vnode
            .dynamic_attributes()
            .filter(move |group| group.parent_element_op_index() == self.op)
    }
}

/// A static template text node viewed through a specific rendered [`VNode`].
#[derive(Clone, Copy)]
pub struct StaticText<'a> {
    vnode: &'a VNode,
    op: usize,
    root_position: Option<usize>,
}

impl<'a> StaticText<'a> {
    pub(crate) fn new(vnode: &'a VNode, op: usize, root_position: Option<usize>) -> Self {
        Self {
            vnode,
            op,
            root_position,
        }
    }

    /// The flat template op for this text node.
    pub fn op(self) -> usize {
        self.op
    }

    /// The static text value.
    pub fn text(self) -> &'static str {
        self.vnode
            .template
            .static_text_at_op(self.op)
            .expect("static text")
    }

    /// The root position when this text node is a vnode root.
    pub fn root_position(self) -> Option<usize> {
        self.root_position
    }
}

/// Iterator over rendered children.
pub struct VNodeChildren<'a> {
    static_children: StaticChildCursor<'a>,
    dynamic_children: DynamicChildCursor<'a>,
    next_static: Option<PositionedChild<'a>>,
    next_dynamic: Option<PositionedChild<'a>>,
}

impl<'a> VNodeChildren<'a> {
    fn roots(vnode: &'a VNode) -> Self {
        let static_children = StaticChildCursor::roots(vnode);
        let dynamic_children = DynamicChildCursor::roots(vnode);
        Self::new(static_children, dynamic_children)
    }

    fn element(element: StaticElement<'a>) -> Self {
        let static_children = StaticChildCursor::element(element);
        let dynamic_children = DynamicChildCursor::element(element);
        Self::new(static_children, dynamic_children)
    }

    fn new(
        mut static_children: StaticChildCursor<'a>,
        mut dynamic_children: DynamicChildCursor<'a>,
    ) -> Self {
        let next_static = static_children.next();
        let next_dynamic = dynamic_children.next();
        Self {
            static_children,
            dynamic_children,
            next_static,
            next_dynamic,
        }
    }

    fn remaining_len(&self) -> usize {
        usize::from(self.next_static.is_some())
            + usize::from(self.next_dynamic.is_some())
            + self.static_children.remaining_len()
            + self.dynamic_children.remaining_len()
    }
}

impl<'a> Iterator for VNodeChildren<'a> {
    type Item = VNodeChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let take_static = match (self.next_static, self.next_dynamic) {
            (Some(static_child), Some(dynamic_child)) => static_child.key() <= dynamic_child.key(),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return None,
        };

        if take_static {
            let child = self.next_static.take().expect("static child checked");
            self.next_static = self.static_children.next();
            Some(child.child)
        } else {
            let child = self.next_dynamic.take().expect("dynamic child checked");
            self.next_dynamic = self.dynamic_children.next();
            Some(child.child)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.remaining_len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for VNodeChildren<'_> {}

#[derive(Clone, Copy)]
struct PositionedChild<'a> {
    position: usize,
    order: usize,
    child: VNodeChild<'a>,
}

impl PositionedChild<'_> {
    fn key(self) -> (usize, usize) {
        (self.position, self.order)
    }
}

#[derive(Clone, Copy)]
enum StaticChildCursor<'a> {
    Roots {
        vnode: &'a VNode,
        op: usize,
        op_count: usize,
        static_root_index: usize,
    },
    Element {
        element: StaticElement<'a>,
        cursor: usize,
        end: usize,
        slot: usize,
    },
}

impl<'a> StaticChildCursor<'a> {
    fn roots(vnode: &'a VNode) -> Self {
        Self::Roots {
            vnode,
            op: 0,
            op_count: vnode.template.decoded_ops().len(),
            static_root_index: 0,
        }
    }

    fn element(element: StaticElement<'a>) -> Self {
        let vnode = element.vnode;
        let cursor = vnode.template.first_child_node_op(element.op).unwrap_or(0);
        let end = vnode.template.element_end(element.op).unwrap_or(0);
        Self::Element {
            element,
            cursor,
            end,
            slot: 0,
        }
    }

    fn next(&mut self) -> Option<PositionedChild<'a>> {
        match self {
            Self::Roots {
                vnode,
                op,
                op_count,
                static_root_index,
            } => {
                while *op < *op_count && !is_static_child(vnode, *op) {
                    *op = vnode.template.next_sibling_op(*op);
                }

                if *op >= *op_count {
                    return None;
                }

                let current_op = *op;
                *op = vnode.template.next_sibling_op(current_op);
                let current_static_root_index = *static_root_index;
                *static_root_index += 1;
                let root_position = vnode
                    .template
                    .root_position_for_static_root(current_static_root_index)
                    .expect("static root position");

                Some(PositionedChild {
                    position: root_position,
                    order: current_static_root_index,
                    child: static_child(vnode, current_op, Some(root_position)),
                })
            }
            Self::Element {
                element,
                cursor,
                end,
                slot,
            } => {
                let vnode = element.vnode;
                while *cursor < *end {
                    let current_op = *cursor;
                    *cursor = vnode.template.next_sibling_op(current_op);
                    if is_static_child(vnode, current_op) {
                        let current_slot = *slot;
                        *slot += 1;
                        return Some(PositionedChild {
                            position: current_slot * 2 + 1,
                            order: current_slot,
                            child: static_child(vnode, current_op, None),
                        });
                    }
                }
                None
            }
        }
    }

    fn remaining_len(self) -> usize {
        let mut remaining = 0;
        let mut cursor = self;
        while cursor.next().is_some() {
            remaining += 1;
        }
        remaining
    }
}

#[derive(Clone, Copy)]
enum DynamicChildCursor<'a> {
    Roots {
        vnode: &'a VNode,
        anchor_index: usize,
        order: usize,
    },
    Element {
        element: StaticElement<'a>,
        anchor_index: usize,
        order: usize,
    },
}

impl<'a> DynamicChildCursor<'a> {
    fn roots(vnode: &'a VNode) -> Self {
        Self::Roots {
            vnode,
            anchor_index: 0,
            order: 0,
        }
    }

    fn element(element: StaticElement<'a>) -> Self {
        Self::Element {
            element,
            anchor_index: 0,
            order: 0,
        }
    }

    fn next(&mut self) -> Option<PositionedChild<'a>> {
        match self {
            Self::Roots {
                vnode,
                anchor_index,
                order,
            } => next_dynamic_child(vnode, anchor_index, order, |group| {
                group.is_root_level().then(|| group.root_position())
            }),
            Self::Element {
                element,
                anchor_index,
                order,
            } => {
                let element_op = element.op;
                next_dynamic_child(element.vnode, anchor_index, order, |group| {
                    (group.parent_element_op_index() == Some(element_op))
                        .then(|| child_position(group.slot_target()))
                })
            }
        }
    }

    fn remaining_len(self) -> usize {
        let mut remaining = 0;
        let mut cursor = self;
        while cursor.next().is_some() {
            remaining += 1;
        }
        remaining
    }
}

fn next_dynamic_child<'a>(
    vnode: &'a VNode,
    anchor_index: &mut usize,
    order: &mut usize,
    mut position: impl FnMut(DynamicNodeGroup<'a>) -> Option<usize>,
) -> Option<PositionedChild<'a>> {
    let anchors = vnode.template.anchors();
    while *anchor_index < anchors.len() {
        let current_anchor_index = *anchor_index;
        *anchor_index += 1;
        let group =
            DynamicNodeGroup::new(vnode, &anchors[current_anchor_index], current_anchor_index);
        if group.is_empty() {
            continue;
        }

        let Some(position) = position(group) else {
            continue;
        };

        let current_order = *order;
        *order += 1;
        return Some(PositionedChild {
            position,
            order: current_order,
            child: VNodeChild::Dynamic(group),
        });
    }

    None
}

/// Effective final attribute value for an element.
#[derive(Clone, Copy)]
pub struct EffectiveAttribute<'a> {
    /// The attribute name.
    pub name: &'static str,
    /// The attribute namespace.
    pub namespace: Option<&'static str>,
    /// The final effective value.
    pub value: EffectiveAttributeValue<'a>,
    /// Whether renderers should always write this attribute.
    pub volatile: bool,
    /// The value source.
    pub source: EffectiveAttributeSource,
}

/// Where an effective attribute value came from.
#[derive(Clone, Copy)]
pub enum EffectiveAttributeValue<'a> {
    /// A static template attribute value.
    Static(&'static str),
    /// A dynamic runtime attribute value.
    Dynamic(&'a AttributeValue),
}

/// The template/runtime source for an effective attribute.
#[derive(Clone, Copy)]
pub enum EffectiveAttributeSource {
    /// A static template attribute.
    Static,
    /// A dynamic runtime attribute.
    Dynamic {
        /// The dynamic value index.
        value_index: usize,
        /// The template anchor index.
        anchor_index: usize,
    },
}

/// Iterator over the final effective attributes for an element.
pub struct ElementAttributes<'a> {
    inner: std::vec::IntoIter<EffectiveAttribute<'a>>,
}

impl<'a> ElementAttributes<'a> {
    fn new(element: StaticElement<'a>) -> Self {
        let mut attributes = Vec::new();
        for (name, value, namespace) in element.vnode.template.static_attrs(element.op) {
            upsert_effective_attribute(
                &mut attributes,
                EffectiveAttribute {
                    name,
                    namespace,
                    value: EffectiveAttributeValue::Static(value),
                    volatile: false,
                    source: EffectiveAttributeSource::Static,
                },
            );
        }

        for group in element.dynamic_attributes() {
            for value_index in group.ids() {
                for attr in element.vnode.dynamic_values[value_index].attrs() {
                    let key = (attr.name, attr.namespace);
                    if matches!(attr.value, AttributeValue::None) {
                        remove_effective_attribute(&mut attributes, key);
                        continue;
                    }

                    upsert_effective_attribute(
                        &mut attributes,
                        EffectiveAttribute {
                            name: attr.name,
                            namespace: attr.namespace,
                            value: EffectiveAttributeValue::Dynamic(&attr.value),
                            volatile: attr.volatile,
                            source: EffectiveAttributeSource::Dynamic {
                                value_index,
                                anchor_index: group.anchor_index(),
                            },
                        },
                    );
                }
            }
        }

        attributes.sort_by_key(|attr| (attr.name, attr.namespace));
        Self {
            inner: attributes.into_iter(),
        }
    }
}

impl<'a> Iterator for ElementAttributes<'a> {
    type Item = EffectiveAttribute<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for ElementAttributes<'_> {}

/// A chunk of dynamic values attached to one template anchor.
#[derive(Clone, Copy)]
pub(crate) enum DynamicChunk<'a> {
    /// Dynamic node values inserted at one template position.
    Nodes(DynamicNodeGroup<'a>),
    /// Dynamic attribute values applied to one static element.
    Attributes(DynamicAttrGroup<'a>),
}

impl<'a> DynamicChunk<'a> {
    fn is_empty(&self) -> bool {
        match self {
            DynamicChunk::Nodes(group) => group.is_empty(),
            DynamicChunk::Attributes(group) => group.is_empty(),
        }
    }
}

impl VNode {
    /// Iterate rendered root children in document order.
    pub fn children(&self) -> VNodeChildren<'_> {
        VNodeChildren::roots(self)
    }

    /// Return the number of root child positions.
    pub fn root_child_count(&self) -> usize {
        self.template.root_position_count()
    }

    /// Iterate dynamic value groups in template document order.
    pub(crate) fn dynamic_groups(&self) -> impl DoubleEndedIterator<Item = DynamicChunk<'_>> + '_ {
        self.template
            .anchors()
            .iter()
            .enumerate()
            .flat_map(|(anchor_index, anchor)| {
                [
                    DynamicChunk::Attributes(DynamicAttrGroup::new(self, anchor, anchor_index)),
                    DynamicChunk::Nodes(DynamicNodeGroup::new(self, anchor, anchor_index)),
                ]
            })
            .filter(|chunk| !chunk.is_empty())
    }

    /// Iterate dynamic node groups in template document order.
    pub fn dynamic_nodes(&self) -> impl DoubleEndedIterator<Item = DynamicNodeGroup<'_>> + '_ {
        self.dynamic_groups().filter_map(|chunk| match chunk {
            DynamicChunk::Nodes(nodes) => Some(nodes),
            DynamicChunk::Attributes(_) => None,
        })
    }

    /// Iterate dynamic attribute groups in template document order.
    pub fn dynamic_attributes(&self) -> impl DoubleEndedIterator<Item = DynamicAttrGroup<'_>> + '_ {
        self.dynamic_groups().filter_map(|chunk| match chunk {
            DynamicChunk::Attributes(attrs) => Some(attrs),
            DynamicChunk::Nodes(_) => None,
        })
    }

    pub(super) fn dynamic_node_slots(
        &self,
    ) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
        self.dynamic_nodes().flat_map(|group| group.slots())
    }

    pub(super) fn dynamic_node_slot(&self, index: usize) -> Option<DynamicNodeSlot<'_>> {
        self.dynamic_node_slots().find(|slot| slot.index() == index)
    }
}

/// A group of dynamic node values that share one insertion anchor.
#[derive(Clone, Copy)]
pub struct DynamicNodeGroup<'a> {
    dynamic_values: &'a [crate::DynamicValue],
    anchor: &'a TemplateAnchor,
    anchor_index: usize,
    root_position: usize,
}

impl<'a> DynamicNodeGroup<'a> {
    pub(super) fn new(vnode: &'a VNode, anchor: &'a TemplateAnchor, anchor_index: usize) -> Self {
        Self {
            dynamic_values: &vnode.dynamic_values,
            anchor,
            anchor_index,
            root_position: vnode
                .template
                .root_position_for_anchor(anchor_index)
                .expect("bad anchor root"),
        }
    }

    /// Iterate the dynamic value indexes in this group.
    pub fn ids(self) -> impl DoubleEndedIterator<Item = usize> + 'a {
        self.anchor
            .values()
            .filter(move |&idx| self.dynamic_values[idx].as_node().is_some())
    }

    fn is_empty(self) -> bool {
        self.ids().next().is_none()
    }

    pub(super) fn slots(self) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'a>> + 'a {
        self.ids().map(move |index| self.slot(index))
    }

    fn slot(self, index: usize) -> DynamicNodeSlot<'a> {
        debug_assert!(self.anchor.values().contains(&index));
        debug_assert!(self.dynamic_values[index].as_node().is_some());
        DynamicNodeSlot { group: self, index }
    }

    /// The static template position where this group is inserted.
    pub fn slot_target(self) -> TemplateSlotTarget {
        self.anchor.slot_target()
    }

    /// The template anchor index for this group.
    pub fn anchor_index(self) -> usize {
        self.anchor_index
    }

    pub(super) fn appends(self) -> bool {
        matches!(self.slot_target(), TemplateSlotTarget::AppendChildren(_))
    }

    /// The root position this group belongs to.
    pub fn root_position(self) -> usize {
        self.root_position
    }

    /// Return true when this dynamic group is inserted at the vnode root level, with no enclosing
    /// static element.
    pub fn is_root_level(self) -> bool {
        match self.slot_target() {
            TemplateSlotTarget::BeforeStatic(path) => path.is_root(),
            TemplateSlotTarget::AppendChildren(path) => path.is_empty(),
        }
    }

    /// The static element op that owns this group, or `None` for root-level groups.
    pub fn parent_element_op_index(self) -> Option<usize> {
        self.anchor.parent_element_op_index()
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.anchor.static_path()
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.slot_target() == other.slot_target()
    }
}

/// One dynamic node value (`index`) viewed over its owning [`TemplateAnchor`].
///
/// An anchor can cover several adjacent node values at the same insertion position (e.g. `{a}{b}`);
/// the diff processes each value separately, so this picks out one `index` from `anchor.values()`.
#[derive(Clone, Copy)]
pub(super) struct DynamicNodeSlot<'a> {
    group: DynamicNodeGroup<'a>,
    index: usize,
}

impl<'a> DynamicNodeSlot<'a> {
    pub(super) fn index(self) -> usize {
        self.index
    }

    pub(super) fn anchor_index(self) -> usize {
        self.group.anchor_index()
    }

    pub(super) fn appends(self) -> bool {
        self.group.appends()
    }

    pub(super) fn root_position(self) -> usize {
        self.group.root_position()
    }

    /// Return true when this dynamic node is inserted at the vnode root level, with no enclosing
    /// static element.
    pub(super) fn is_root_level(self) -> bool {
        self.group.is_root_level()
    }

    pub(super) fn parent_path(self) -> TemplatePath {
        self.group.parent_path()
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.group.shares_insertion_position(other.group)
    }
}

/// A group of dynamic attribute values that all attach to one static element, viewed directly over
/// its [`TemplateAnchor`].
#[derive(Clone, Copy)]
pub struct DynamicAttrGroup<'a> {
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

    /// Iterate the dynamic value indexes in this group.
    pub fn ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.anchor
            .values()
            .filter(|&idx| self.dynamic_values[idx].as_attrs().is_some())
    }

    fn is_empty(&self) -> bool {
        self.ids().next().is_none()
    }

    /// The template anchor index for the static element this group applies to.
    pub fn anchor_index(&self) -> usize {
        self.anchor_index
    }

    /// The static template path for the element this group applies to.
    pub fn static_path(&self) -> TemplatePath {
        self.anchor.static_path()
    }

    /// The static element op this group applies to.
    pub fn parent_element_op_index(&self) -> usize {
        self.anchor
            .parent_element_op_index()
            .expect("bad attr anchor")
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

fn static_child<'a>(vnode: &'a VNode, op: usize, root_position: Option<usize>) -> VNodeChild<'a> {
    if vnode.template.element_meta_at_op(op).is_some() {
        VNodeChild::Element(StaticElement::new(vnode, op, root_position))
    } else if vnode.template.static_text_at_op(op).is_some() {
        VNodeChild::Text(StaticText::new(vnode, op, root_position))
    } else {
        unreachable!("static child must start at an element or static text op")
    }
}

fn is_static_child(vnode: &VNode, op: usize) -> bool {
    vnode.template.element_meta_at_op(op).is_some()
        || vnode.template.static_text_at_op(op).is_some()
}

fn child_position(target: TemplateSlotTarget) -> usize {
    match target {
        TemplateSlotTarget::BeforeStatic(path) => path.split_insertion().1 * 2,
        TemplateSlotTarget::AppendChildren(_) => usize::MAX,
    }
}

fn upsert_effective_attribute<'a>(
    attributes: &mut Vec<EffectiveAttribute<'a>>,
    attribute: EffectiveAttribute<'a>,
) {
    let key = (attribute.name, attribute.namespace);
    match attributes
        .iter_mut()
        .find(|existing| (existing.name, existing.namespace) == key)
    {
        Some(existing) => *existing = attribute,
        None => attributes.push(attribute),
    }
}

fn remove_effective_attribute(
    attributes: &mut Vec<EffectiveAttribute<'_>>,
    key: (&'static str, Option<&'static str>),
) {
    if let Some(index) = attributes
        .iter()
        .position(|attr| (attr.name, attr.namespace) == key)
    {
        attributes.remove(index);
    }
}
