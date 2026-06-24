use crate::{Attribute, DynamicNode, VNode};
use dioxus_core_template::{
    StaticTemplateElement, StaticTemplateNode, StaticTemplateNodeIter, StaticTemplateText,
    TemplateAnchor, TemplatePath, TemplateSlotTarget,
};

/// A rendered child of a [`VNode`] or a static template element.
#[derive(Clone, Copy)]
pub enum VNodeChild<'a> {
    /// A static template element.
    Element(StaticElement<'a>),
    /// A static template text node.
    Text(StaticText<'a>),
    /// One or more dynamic node values at the same insertion position.
    Dynamic(DynamicAnchor<'a>),
}

/// A static template element viewed through a specific rendered [`VNode`].
#[derive(Clone, Copy)]
pub struct StaticElement<'a> {
    vnode: &'a VNode,
    op: usize,
    path: TemplatePath,
    anchor_idx: Option<usize>,
}

impl<'a> StaticElement<'a> {
    fn new(vnode: &'a VNode, op: usize, path: TemplatePath, anchor_idx: Option<usize>) -> Self {
        Self {
            vnode,
            op,
            path,
            anchor_idx,
        }
    }

    /// The flat template op for this element.
    pub fn op(self) -> usize {
        self.op
    }

    fn template_element(self) -> StaticTemplateElement<'a> {
        self.vnode
            .template()
            .static_element(self.op)
            .expect("static element")
    }

    /// The element tag.
    pub fn tag(self) -> &'static str {
        self.template_element().tag()
    }

    /// The element namespace.
    pub fn namespace(self) -> Option<&'static str> {
        self.template_element().namespace()
    }

    /// The structural anchor for this static node when it has one.
    pub fn anchor_index(self) -> Option<usize> {
        self.anchor_idx
    }

    /// Iterate static template attributes for this element.
    pub fn static_attributes(
        self,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + 'a {
        self.template_element()
            .attributes()
            .map(|attr| (attr.name, attr.value, attr.namespace))
    }

    /// Iterate rendered children for this element.
    pub fn children(self) -> impl ExactSizeIterator<Item = VNodeChild<'a>> + 'a {
        VNodeChildren::element(self)
    }

    /// Return true if this element has any rendered child.
    pub fn has_children(self) -> bool {
        self.children().next().is_some()
    }

    /// Iterate dynamic anchors with attributes that target this element.
    pub fn dynamic_anchors(self) -> impl Iterator<Item = DynamicAnchor<'a>> + 'a {
        self.vnode.dynamic_anchors().filter(move |anchor| {
            anchor.parent_element_op_index() == Some(self.op) && anchor.attrs().len() > 0
        })
    }
}

/// A static template text node viewed through a specific rendered [`VNode`].
#[derive(Clone, Copy)]
pub struct StaticText<'a> {
    vnode: &'a VNode,
    op: usize,
    anchor_idx: Option<usize>,
}

impl<'a> StaticText<'a> {
    fn new(vnode: &'a VNode, op: usize, anchor_idx: Option<usize>) -> Self {
        Self {
            vnode,
            op,
            anchor_idx,
        }
    }

    /// The flat template op for this text node.
    pub fn op(self) -> usize {
        self.op
    }

    fn template_text(self) -> StaticTemplateText<'a> {
        self.vnode
            .template()
            .static_text(self.op)
            .expect("static text")
    }

    /// The static text value.
    pub fn text(self) -> &'static str {
        self.template_text().text()
    }

    /// The structural anchor for this static node when it has one.
    pub fn anchor_index(self) -> Option<usize> {
        self.anchor_idx
    }
}

/// Iterator over rendered children.
pub(crate) struct VNodeChildren<'a> {
    inner: VNodeChildrenInner<'a>,
}

#[derive(Clone, Copy)]
pub(super) struct StaticAnchorTarget {
    pub(super) anchor_index: usize,
    pub(super) path: TemplatePath,
}

impl<'a> VNodeChildren<'a> {
    fn roots(vnode: &'a VNode) -> Self {
        Self {
            inner: VNodeChildrenInner::Roots(RootChildCursor {
                vnode,
                anchor_index: 0,
                pending_static: None,
            }),
        }
    }

    fn element(element: StaticElement<'a>) -> Self {
        Self {
            inner: VNodeChildrenInner::Element(ElementChildCursor::new(element)),
        }
    }

    fn remaining_len(&self) -> usize {
        self.inner.remaining_len()
    }
}

impl<'a> Iterator for VNodeChildren<'a> {
    type Item = VNodeChild<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.remaining_len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for VNodeChildren<'_> {}

#[derive(Clone, Copy)]
enum VNodeChildrenInner<'a> {
    Roots(RootChildCursor<'a>),
    Element(ElementChildCursor<'a>),
}

impl<'a> VNodeChildrenInner<'a> {
    fn next(&mut self) -> Option<VNodeChild<'a>> {
        match self {
            Self::Roots(cursor) => cursor.next(),
            Self::Element(cursor) => cursor.next(),
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
struct RootChildCursor<'a> {
    vnode: &'a VNode,
    anchor_index: usize,
    pending_static: Option<(usize, StaticTemplateNode<'a>)>,
}

impl<'a> RootChildCursor<'a> {
    fn next(&mut self) -> Option<VNodeChild<'a>> {
        if let Some((anchor_index, node)) = self.pending_static.take() {
            let path = self.vnode.template().anchors()[anchor_index].static_path();
            return Some(static_child(self.vnode, node, path, Some(anchor_index)));
        }

        while self.anchor_index < self.vnode.template().anchors().len() {
            let anchor_index = self.anchor_index;
            self.anchor_index += 1;
            let anchor = DynamicAnchor::new(self.vnode, anchor_index);
            if !anchor.is_root_level() {
                continue;
            }

            let static_root = (!anchor.is_last_static_node() && anchor.static_path().is_root())
                .then(|| {
                    self.vnode
                        .template()
                        .static_node_at_path(anchor.static_path())
                        .expect("static root anchor")
                });

            if anchor.nodes().len() > 0 {
                self.pending_static = static_root.map(|node| (anchor_index, node));
                return Some(VNodeChild::Dynamic(anchor));
            }

            if let Some(node) = static_root {
                let path = self.vnode.template().anchors()[anchor_index].static_path();
                return Some(static_child(self.vnode, node, path, Some(anchor_index)));
            }
        }

        None
    }
}

#[derive(Clone, Copy)]
struct ElementChildCursor<'a> {
    static_children: StaticChildCursor<'a>,
    dynamic_children: DynamicChildCursor<'a>,
    next_static: Option<PositionedChild<'a>>,
    next_dynamic: Option<PositionedChild<'a>>,
}

impl<'a> ElementChildCursor<'a> {
    fn new(element: StaticElement<'a>) -> Self {
        let mut static_children = StaticChildCursor::new(element);
        let mut dynamic_children = DynamicChildCursor::new(element);
        let next_static = static_children.next();
        let next_dynamic = dynamic_children.next();
        Self {
            static_children,
            dynamic_children,
            next_static,
            next_dynamic,
        }
    }

    fn next(&mut self) -> Option<VNodeChild<'a>> {
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
}

#[derive(Clone, Copy)]
struct PositionedChild<'a> {
    position: usize,
    order: usize,
    child: VNodeChild<'a>,
}

impl<'a> PositionedChild<'a> {
    fn key(self) -> (usize, usize) {
        (self.position, self.order)
    }
}

#[derive(Clone, Copy)]
struct StaticChildCursor<'a> {
    vnode: &'a VNode,
    nodes: StaticTemplateNodeIter<'a>,
    path: TemplatePath,
    slot: usize,
}

impl<'a> StaticChildCursor<'a> {
    fn new(element: StaticElement<'a>) -> Self {
        let vnode = element.vnode;
        Self {
            vnode,
            nodes: element.template_element().children(),
            path: element.path.next_child(),
            slot: 0,
        }
    }

    fn next(&mut self) -> Option<PositionedChild<'a>> {
        let node = self.nodes.next()?;
        let path = self.path;
        let current_slot = self.slot;
        let anchor_idx = self.vnode.static_anchor_index_for_path(path);
        self.path = self.path.next_sibling();
        self.slot += 1;
        Some(PositionedChild {
            position: current_slot * 2 + 1,
            order: current_slot,
            child: static_child(self.vnode, node, path, anchor_idx),
        })
    }
}

#[derive(Clone, Copy)]
struct DynamicChildCursor<'a> {
    element: StaticElement<'a>,
    anchor_index: usize,
    order: usize,
}

impl<'a> DynamicChildCursor<'a> {
    fn new(element: StaticElement<'a>) -> Self {
        Self {
            element,
            anchor_index: 0,
            order: 0,
        }
    }

    fn next(&mut self) -> Option<PositionedChild<'a>> {
        let element_op = self.element.op;
        next_dynamic_child(
            self.element.vnode,
            &mut self.anchor_index,
            &mut self.order,
            |anchor| {
                (anchor.parent_element_op_index() == Some(element_op))
                    .then(|| child_position(anchor.slot_target()))
            },
        )
    }
}

fn next_dynamic_child<'a>(
    vnode: &'a VNode,
    anchor_index: &mut usize,
    order: &mut usize,
    mut position: impl FnMut(DynamicAnchor<'a>) -> Option<usize>,
) -> Option<PositionedChild<'a>> {
    while *anchor_index < vnode.template().anchors().len() {
        let current_anchor_index = *anchor_index;
        *anchor_index += 1;
        let anchor = DynamicAnchor::new(vnode, current_anchor_index);
        if anchor.nodes().len() == 0 {
            continue;
        }

        let Some(position) = position(anchor) else {
            continue;
        };

        let current_order = *order;
        *order += 1;
        return Some(PositionedChild {
            position,
            order: current_order,
            child: VNodeChild::Dynamic(anchor),
        });
    }

    None
}

impl VNode {
    /// Iterate rendered root children in document order.
    pub fn children(&self) -> impl ExactSizeIterator<Item = VNodeChild<'_>> + '_ {
        VNodeChildren::roots(self)
    }

    /// Return the number of root child positions.
    pub fn root_child_count(&self) -> usize {
        self.children().len()
    }

    /// Iterate dynamic anchors in template document order.
    pub fn dynamic_anchors(&self) -> impl DoubleEndedIterator<Item = DynamicAnchor<'_>> + '_ {
        (0..self.template().anchors().len())
            .map(|anchor_index| DynamicAnchor::new(self, anchor_index))
    }

    pub(super) fn dynamic_node_slots(
        &self,
    ) -> impl DoubleEndedIterator<Item = DynamicNodeSlot<'_>> + '_ {
        self.dynamic_anchors().flat_map(|anchor| anchor.nodes())
    }

    pub(super) fn dynamic_anchor(&self, anchor_index: usize) -> DynamicAnchor<'_> {
        DynamicAnchor::new(self, anchor_index)
    }

    fn static_anchor_index_for_path(&self, path: TemplatePath) -> Option<usize> {
        self.template()
            .anchors()
            .iter()
            .position(|anchor| anchor.static_path() == path)
    }

    pub(super) fn dynamic_node_slots_after<'a>(
        &'a self,
        slot: DynamicNodeSlot<'a>,
    ) -> impl Iterator<Item = DynamicNodeSlot<'a>> + 'a {
        let anchor = slot.anchor();
        let anchor_index = anchor.anchor_index();
        let current_anchor_slots = ((slot.index() + 1)..anchor.template_anchor().nodes().end)
            .map(move |index| DynamicNodeSlot { anchor, index });
        let later_anchor_slots = ((anchor_index + 1)..self.template().anchors().len())
            .flat_map(move |anchor_index| DynamicAnchor::new(self, anchor_index).nodes());

        current_anchor_slots.chain(later_anchor_slots)
    }

    pub(super) fn dynamic_node_slots_after_sharing_insertion_position<'a>(
        &'a self,
        slot: DynamicNodeSlot<'a>,
    ) -> impl Iterator<Item = DynamicNodeSlot<'a>> + 'a {
        self.dynamic_node_slots_after(slot)
            .map_while(move |sibling| {
                if !sibling.has_same_insertion_parent(slot) {
                    Some(None)
                } else if sibling.shares_insertion_position(slot) {
                    Some(Some(sibling))
                } else {
                    None
                }
            })
            .flatten()
    }

    pub(super) fn static_anchor_targets_under(
        &self,
        root_anchor_index: usize,
    ) -> impl Iterator<Item = StaticAnchorTarget> + '_ {
        let root_path = self.template().anchors()[root_anchor_index].static_path();
        self.template()
            .anchors()
            .iter()
            .enumerate()
            .filter_map(move |(anchor_index, anchor)| {
                let path = anchor.static_path();
                (!path.is_empty() && path.starts_with(root_path))
                    .then_some(StaticAnchorTarget { anchor_index, path })
            })
    }
}

/// A dynamic template anchor viewed through a rendered [`VNode`].
#[derive(Clone, Copy)]
pub struct DynamicAnchor<'a> {
    vnode: &'a VNode,
    anchor_index: usize,
}

impl<'a> DynamicAnchor<'a> {
    pub(super) fn new(vnode: &'a VNode, anchor_index: usize) -> Self {
        Self {
            vnode,
            anchor_index,
        }
    }

    fn template_anchor(self) -> &'a TemplateAnchor {
        &self.vnode.template().anchors()[self.anchor_index]
    }

    /// Iterate the dynamic node slots owned by this anchor.
    pub fn nodes(
        self,
    ) -> impl ExactSizeIterator<Item = DynamicNodeSlot<'a>> + DoubleEndedIterator + 'a {
        self.template_anchor()
            .nodes()
            .map(move |index| DynamicNodeSlot {
                anchor: self,
                index,
            })
    }

    /// Iterate the dynamic attribute slots owned by this anchor.
    pub fn attrs(
        self,
    ) -> impl ExactSizeIterator<Item = DynamicAttrSlot<'a>> + DoubleEndedIterator + 'a {
        self.template_anchor()
            .attributes()
            .map(move |index| DynamicAttrSlot {
                anchor: self,
                index,
            })
    }

    /// The static template position where this anchor is inserted.
    pub fn slot_target(self) -> TemplateSlotTarget {
        self.template_anchor().slot_target()
    }

    pub(crate) fn anchor_index(self) -> usize {
        self.anchor_index
    }

    /// Whether this anchor points at the last static node at its sibling level.
    pub fn is_last_static_node(self) -> bool {
        self.slot_target().is_last_static_node()
    }

    pub(crate) fn is_parent_append_target(self) -> bool {
        self.template_anchor().is_parent_append_target()
    }

    /// Return true when this dynamic anchor is inserted at the vnode root level, with no enclosing
    /// static element.
    pub fn is_root_level(self) -> bool {
        self.parent_element_op_index().is_none()
    }

    /// The static element op that owns this anchor, or `None` for root-level anchors.
    pub fn parent_element_op_index(self) -> Option<usize> {
        self.template_anchor().parent_element_op_index()
    }

    /// The static template path for this anchor.
    pub fn static_path(self) -> TemplatePath {
        self.template_anchor().static_path()
    }

    fn has_same_insertion_parent(self, other: Self) -> bool {
        self.parent_element_op_index() == other.parent_element_op_index()
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.has_same_insertion_parent(other) && self.slot_target() == other.slot_target()
    }

    pub(super) fn static_attr_value_for_key(
        self,
        key: (&'static str, Option<&'static str>),
    ) -> Option<&'static str> {
        let element_op = self
            .template_anchor()
            .parent_element_op_index()
            .expect("bad attr anchor");
        self.vnode
            .template()
            .static_element(element_op)?
            .attribute_value(key)
    }
}

#[derive(Clone, Copy)]
/// A dynamic node slot owned by a [`DynamicAnchor`].
pub struct DynamicNodeSlot<'a> {
    anchor: DynamicAnchor<'a>,
    index: usize,
}

impl<'a> DynamicNodeSlot<'a> {
    /// The dynamic anchor that owns this node slot.
    pub fn anchor(self) -> DynamicAnchor<'a> {
        self.anchor
    }

    pub(crate) fn index(self) -> usize {
        self.index
    }

    pub(super) fn has_same_insertion_parent(self, other: Self) -> bool {
        self.anchor.has_same_insertion_parent(other.anchor)
    }

    pub(super) fn shares_insertion_position(self, other: Self) -> bool {
        self.anchor.shares_insertion_position(other.anchor)
    }
}

impl std::ops::Deref for DynamicNodeSlot<'_> {
    type Target = DynamicNode;

    fn deref(&self) -> &Self::Target {
        &self.anchor.vnode.dynamic_node_values()[self.index]
    }
}

#[derive(Clone, Copy)]
/// A dynamic attribute slot owned by a [`DynamicAnchor`].
pub struct DynamicAttrSlot<'a> {
    anchor: DynamicAnchor<'a>,
    index: usize,
}

impl<'a> DynamicAttrSlot<'a> {
    /// The dynamic attributes for this slot.
    pub fn attrs(self) -> &'a [Attribute] {
        self.anchor.vnode.dynamic_attr_values()[self.index].as_ref()
    }

    /// The dynamic anchor that owns this attribute slot.
    pub fn anchor(self) -> DynamicAnchor<'a> {
        self.anchor
    }

    pub(crate) fn index(self) -> usize {
        self.index
    }
}

fn static_child<'a>(
    vnode: &'a VNode,
    node: StaticTemplateNode<'_>,
    path: TemplatePath,
    anchor_idx: Option<usize>,
) -> VNodeChild<'a> {
    match node {
        StaticTemplateNode::Element(element) => {
            VNodeChild::Element(StaticElement::new(vnode, element.op(), path, anchor_idx))
        }
        StaticTemplateNode::Text(text) => {
            VNodeChild::Text(StaticText::new(vnode, text.op(), anchor_idx))
        }
    }
}

fn child_position(target: TemplateSlotTarget) -> usize {
    if target.is_last_static_node() {
        usize::MAX
    } else {
        target.static_path().split_insertion().1 * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attribute, DynamicNode, DynamicValues};
    use dioxus_core_template::RuntimeTemplateBuilder;

    fn vnode_from_builder(
        builder: RuntimeTemplateBuilder,
        dynamic_nodes: usize,
        dynamic_attrs: usize,
    ) -> VNode {
        VNode::new(
            builder.finish(),
            DynamicValues::from_parts(
                None,
                (0..dynamic_nodes)
                    .map(|_| DynamicNode::Fragment(Vec::new()))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                (0..dynamic_attrs)
                    .map(|_| vec![Attribute::new("class", "value", None, false)].into_boxed_slice())
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        )
    }

    #[test]
    fn nested_trailing_dynamic_before_root_dynamic_is_parent_append_target() {
        let mut builder = RuntimeTemplateBuilder::default();
        builder.open_element("tag0", None);
        builder.dynamic_node(false);
        builder.close_element();
        builder.dynamic_node(false);
        let template = builder.finish();

        let vnode = VNode::new(
            template,
            DynamicValues::from_parts(
                None,
                Box::new([
                    DynamicNode::Fragment(Vec::new()),
                    DynamicNode::Fragment(Vec::new()),
                ]),
                Box::new([]),
            ),
        );

        let anchor = vnode
            .dynamic_anchors()
            .find(|anchor| anchor.parent_element_op_index().is_some() && anchor.nodes().len() > 0)
            .expect("nested dynamic anchor");

        assert!(anchor.is_parent_append_target());
    }

    #[test]
    fn nested_static_child_after_dynamic_keeps_anchor_index() {
        let mut builder = RuntimeTemplateBuilder::default();
        builder.open_element("root", None);
        builder.dynamic_node(true);
        builder.open_element("child", None);
        builder.close_element();
        builder.close_element();

        let vnode = vnode_from_builder(builder, 1, 0);
        let root = vnode
            .children()
            .find_map(|child| match child {
                VNodeChild::Element(element) => Some(element),
                _ => None,
            })
            .expect("root element");

        let mut children = root.children();
        let dynamic_anchor_index = match children.next().expect("dynamic child") {
            VNodeChild::Dynamic(anchor) => anchor.anchor_index(),
            _ => panic!("expected dynamic child before static child"),
        };
        let static_anchor_index = match children.next().expect("static child") {
            VNodeChild::Element(element) => element.anchor_index(),
            _ => panic!("expected static child"),
        };

        assert_eq!(static_anchor_index, Some(dynamic_anchor_index));
    }

    #[test]
    fn nested_trailing_dynamic_targets_parent_append() {
        let mut builder = RuntimeTemplateBuilder::default();
        builder.open_element("root", None);
        builder.open_element("child", None);
        builder.close_element();
        builder.dynamic_node(false);
        builder.close_element();

        let vnode = vnode_from_builder(builder, 1, 0);
        let root = vnode
            .children()
            .find_map(|child| match child {
                VNodeChild::Element(element) => Some(element),
                _ => None,
            })
            .expect("root element");

        let mut children = root.children();
        let static_anchor_index = match children.next().expect("static child") {
            VNodeChild::Element(element) => element.anchor_index(),
            _ => panic!("expected static child"),
        };
        let dynamic_anchor = match children.next().expect("dynamic child") {
            VNodeChild::Dynamic(anchor) => anchor,
            _ => panic!("expected trailing dynamic child"),
        };

        assert_eq!(static_anchor_index, None);
        assert!(dynamic_anchor.is_parent_append_target());
    }
}
