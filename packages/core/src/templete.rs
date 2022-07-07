use std::{cell::Cell, hash::Hash, marker::PhantomData};

use bumpalo::Bump;

use crate::{
    diff::DiffState,
    dynamic_template_context::{DynamicNodeMapping, TemplateContext},
    innerlude::GlobalNodeId,
    Attribute, AttributeValue, ElementId, Mutations,
};

/// An Template's unique identifier.
///
/// `TemplateId` is a `usize` that is unique across the entire VirtualDOM and across time.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TemplateId(pub usize);

impl Into<u64> for TemplateId {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

/// A TemplateNode's unique identifier.
///
/// `TemplateNodeId` is a `usize` that is only unique across the template that contains it, it is not unique across multaple instances of that template.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TemplateNodeId(pub usize);

impl Into<u64> for TemplateNodeId {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

/// A refrence to a template along with any context needed to hydrate it
pub struct VTemplateRef<'a> {
    pub(crate) id: Cell<Option<ElementId>>,
    pub(crate) template: &'a Template<'a>,
    pub(crate) dynamic_context: &'a TemplateContext<'a>,
}

impl<'a> VTemplateRef<'a> {
    pub(crate) fn hydrate<'b>(&self, diff_state: &mut DiffState<'a>) {
        fn hydrate_inner<
            'b,
            Nodes,
            Attributes,
            V,
            Children,
            Fragment,
            Listeners,
            TextSegments,
            Text,
        >(
            nodes: &'b Nodes,
            ctx: (&mut DiffState<'b>, &VTemplateRef<'b>),
        ) where
            Nodes: AsRef<
                [TemplateNode<Attributes, V, Children, Fragment, Listeners, TextSegments, Text>],
            >,
            Attributes: AsRef<[TemplateAttribute<V>]>,
            V: TemplateValue,
            Children: AsRef<[TemplateNodeId]>,
            Fragment: AsRef<[TemplateNodeId]>,
            Listeners: AsRef<[usize]>,
            TextSegments: AsRef<[TextTemplateSegment<Text>]>,
            Text: AsRef<str>,
        {
            let (diff_state, template_ref) = ctx;
            for id in template_ref.template.dynamic_ids.all_dynamic() {
                let dynamic_node = &nodes.as_ref()[id.0];
                let real_id = template_ref.id.get().unwrap();
                diff_state.element_stack.push(GlobalNodeId::TemplateId {
                    template_ref_id: real_id,
                    template_id: id,
                });
                match &dynamic_node.node_type {
                    TemplateNodeType::Element(el) => {
                        let TemplateElement {
                            tag,
                            namespace,
                            attributes,
                            children,
                            listeners,
                            ..
                        } = el;
                        for attr in attributes.as_ref() {
                            if let TemplateAttributeValue::Dynamic(idx) = attr.value {
                                let attribute = Attribute {
                                    name: attr.name,
                                    value: template_ref
                                        .dynamic_context
                                        .resolve_attribute(idx)
                                        .to_owned(),
                                    is_static: false,
                                    is_volatile: false,
                                    namespace: attr.namespace,
                                };
                                let scope_bump = diff_state.current_scope_bump();
                                diff_state
                                    .mutations
                                    .set_attribute(scope_bump.alloc(attribute), id);
                            }
                        }
                        for listener_idx in listeners.as_ref() {
                            let listener =
                                template_ref.dynamic_context.resolve_listener(*listener_idx);
                            let global_id = GlobalNodeId::TemplateId {
                                template_ref_id: real_id,
                                template_id: id,
                            };
                            listener.mounted_node.set(Some(global_id));
                            diff_state
                                .mutations
                                .new_event_listener(listener, diff_state.current_scope());
                        }
                        let mut children_created = 0;
                        for child in children.as_ref() {
                            let node = &nodes.as_ref()[child.0];
                            if let TemplateNodeType::DynamicNode(idx) = node.node_type {
                                diff_state.create_node(&template_ref.dynamic_context.nodes[idx]);
                                children_created += 1;
                            }
                        }
                        if children_created > 0 {
                            diff_state.mutations.push_root(id);
                            diff_state.mutations.append_children(children_created);
                            diff_state.mutations.pop_root();
                        }
                    }
                    TemplateNodeType::Text(text) => {
                        let new_text = template_ref
                            .dynamic_context
                            .resolve_text(&text.segments.as_ref());
                        let scope_bump = diff_state.current_scope_bump();
                        diff_state
                            .mutations
                            .set_text(scope_bump.alloc(new_text), id)
                    }
                    TemplateNodeType::DynamicNode(idx) => {
                        // this will only be triggered for root elements
                        diff_state.create_node(&template_ref.dynamic_context.nodes[*idx]);
                    }
                    _ => {
                        todo!()
                    }
                }
                diff_state.element_stack.pop();
            }
        }
        self.template
            .nodes
            .with_nodes(hydrate_inner, hydrate_inner, (diff_state, self));
    }
}

#[derive(Debug)]
pub(crate) struct Template<'b> {
    pub(crate) id: TemplateId,
    pub(crate) nodes: TemplateNodes,
    /// Any nodes that contain dynamic components. This is stored in the tmeplate to avoid traversing the tree every time a template is refrenced.
    pub(crate) dynamic_ids: DynamicNodeMapping<'b>,
}

impl<'b> Template<'b> {
    pub(crate) fn create(&mut self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: TemplateId) {
        mutations.create_templete(id.into());
        let id = TemplateNodeId(0);
        if !self.nodes.is_empty() {
            self.create_node(mutations, bump, id);
        }
        mutations.finish_templete();
    }

    fn create_node(&self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: TemplateNodeId) {
        fn crate_node_inner<'b, Attributes, V, Children, Fragment, Listeners, TextSegments, Text>(
            node: &TemplateNode<Attributes, V, Children, Fragment, Listeners, TextSegments, Text>,
            ctx: (&mut Mutations<'b>, &'b Bump, &Template<'b>),
        ) where
            Attributes: AsRef<[TemplateAttribute<V>]>,
            V: TemplateValue,
            Children: AsRef<[TemplateNodeId]>,
            Fragment: AsRef<[TemplateNodeId]>,
            Listeners: AsRef<[usize]>,
            TextSegments: AsRef<[TextTemplateSegment<Text>]>,
            Text: AsRef<str>,
        {
            let (mutations, bump, template) = ctx;
            let id = node.id;
            match &node.node_type {
                TemplateNodeType::Element(el) => {
                    let TemplateElement {
                        tag,
                        namespace,
                        attributes,
                        children,
                        listeners,
                        ..
                    } = el;
                    mutations.create_element(tag, *namespace, id);
                    for attr in attributes.as_ref() {
                        if let TemplateAttributeValue::Static(val) = &attr.value {
                            let val: AttributeValue<'b> = val.allocate(bump);
                            let attribute = Attribute {
                                name: attr.name,
                                value: val,
                                is_static: true,
                                is_volatile: false,
                                namespace: attr.namespace,
                            };
                            mutations.set_attribute(bump.alloc(attribute), id);
                        }
                    }
                    let mut children_created = 0;
                    for child in children.as_ref() {
                        template.create_node(mutations, bump, *child);
                        children_created += 1;
                    }

                    mutations.append_children(children_created);
                }
                TemplateNodeType::Text(text) => {
                    let mut text_iter = text.segments.as_ref().into_iter();
                    if let (Some(TextTemplateSegment::Static(txt)), None) =
                        (text_iter.next(), text_iter.next())
                    {
                        mutations.create_text_node(bump.alloc_str(txt.as_ref()), id);
                    } else {
                        mutations.create_text_node("", id);
                    }
                }
                TemplateNodeType::DynamicNode(_) => {
                    mutations.create_placeholder(id);
                }
                TemplateNodeType::Fragment(nodes) => {
                    for node in nodes.as_ref() {
                        template.create_node(mutations, bump, *node);
                    }
                }
            }
        }
        self.nodes.with_node(
            id,
            crate_node_inner,
            crate_node_inner,
            (mutations, bump, self),
        );
    }
}

#[derive(Debug)]
pub(crate) enum TemplateNodes {
    Static(&'static [StaticTemplateNode]),
    Owned(Vec<OwnedTemplateNode>),
}

impl TemplateNodes {
    fn is_empty(&self) -> bool {
        match self {
            TemplateNodes::Static(nodes) => nodes.is_empty(),
            TemplateNodes::Owned(nodes) => nodes.is_empty(),
        }
    }

    fn to_owned(&mut self) {
        todo!()
    }

    pub(crate) fn with_node<F1, F2, Ctx, R>(
        &self,
        id: TemplateNodeId,
        mut f1: F1,
        mut f2: F2,
        ctx: Ctx,
    ) -> R
    where
        F1: FnMut(&StaticTemplateNode, Ctx) -> R,
        F2: FnMut(&OwnedTemplateNode, Ctx) -> R,
    {
        match self {
            TemplateNodes::Static(nodes) => f1(&nodes[id.0], ctx),
            TemplateNodes::Owned(nodes) => f2(&nodes[id.0], ctx),
        }
    }

    pub(crate) fn with_nodes<'a, F1, F2, Ctx>(&'a self, mut f1: F1, mut f2: F2, ctx: Ctx)
    where
        F1: FnMut(&'a &'static [StaticTemplateNode], Ctx),
        F2: FnMut(&'a Vec<OwnedTemplateNode>, Ctx),
    {
        match self {
            TemplateNodes::Static(nodes) => f1(&nodes, ctx),
            TemplateNodes::Owned(nodes) => f2(&nodes, ctx),
        }
    }
}

type StaticTemplateNode = TemplateNode<
    &'static [TemplateAttribute<AttributeValue<'static>>],
    AttributeValue<'static>,
    &'static [TemplateNodeId],
    &'static [TemplateNodeId],
    &'static [usize],
    &'static [TextTemplateSegment<&'static str>],
    &'static str,
>;

type OwnedTemplateNode = TemplateNode<
    Vec<TemplateAttribute<OwnedTemplateValue>>,
    OwnedTemplateValue,
    Vec<TemplateNodeId>,
    Vec<TemplateNodeId>,
    Vec<usize>,
    Vec<TextTemplateSegment<String>>,
    String,
>;

/// Templates can only contain a limited subset of VNodes and keys are not needed, as diffing will be skipped.
/// Dynamic parts of the Template are inserted into the VNode using the `TemplateContext` by traversing the tree in order and filling in dynamic parts
/// This template node is generic over the storage of the nodes to allow for owned and &'static versions.
#[derive(Debug)]
pub(crate) struct TemplateNode<Attributes, V, Children, Fragment, Listeners, TextSegments, Text>
where
    Attributes: AsRef<[TemplateAttribute<V>]>,
    V: TemplateValue,
    Children: AsRef<[TemplateNodeId]>,
    Fragment: AsRef<[TemplateNodeId]>,
    Listeners: AsRef<[usize]>,
    TextSegments: AsRef<[TextTemplateSegment<Text>]>,
    Text: AsRef<str>,
{
    /// The ID of the [`TemplateNode`]. Note that this is not an elenemt id, and should be allocated seperately from VNodes on the frontend.
    pub id: TemplateNodeId,
    pub node_type:
        TemplateNodeType<Attributes, V, Children, Listeners, TextSegments, Text, Fragment>,
}

#[derive(Debug)]
pub(crate) struct TemplateAttribute<V: TemplateValue> {
    pub name: &'static str,
    pub namespace: Option<&'static str>,
    pub value: TemplateAttributeValue<V>,
}

#[derive(Debug)]
pub(crate) enum TemplateAttributeValue<V: TemplateValue> {
    Static(V),
    Dynamic(usize),
}

pub(crate) trait TemplateValue {
    fn allocate<'b>(&self, bump: &'b Bump) -> AttributeValue<'b>;
}

impl TemplateValue for AttributeValue<'static> {
    fn allocate<'b>(&self, bump: &'b Bump) -> AttributeValue<'b> {
        match self.clone() {
            AttributeValue::Text(txt) => AttributeValue::Text(bump.alloc_str(txt)),
            AttributeValue::Bytes(bytes) => AttributeValue::Bytes(bump.alloc_slice_copy(bytes)),
            AttributeValue::Float32(f) => AttributeValue::Float32(f),
            AttributeValue::Float64(f) => AttributeValue::Float64(f),
            AttributeValue::Int32(i) => AttributeValue::Int32(i),
            AttributeValue::Int64(i) => AttributeValue::Int64(i),
            AttributeValue::Uint32(u) => AttributeValue::Uint32(u),
            AttributeValue::Uint64(u) => AttributeValue::Uint64(u),
            AttributeValue::Bool(b) => AttributeValue::Bool(b),
            AttributeValue::Vec3Float(f1, f2, f3) => AttributeValue::Vec3Float(f1, f2, f3),
            AttributeValue::Vec3Int(i1, i2, i3) => AttributeValue::Vec3Int(i1, i2, i3),
            AttributeValue::Vec3Uint(u1, u2, u3) => AttributeValue::Vec3Uint(u1, u2, u3),
            AttributeValue::Vec4Float(f1, f2, f3, f4) => AttributeValue::Vec4Float(f1, f2, f3, f4),
            AttributeValue::Vec4Int(i1, i2, i3, i4) => AttributeValue::Vec4Int(i1, i2, i3, i4),
            AttributeValue::Vec4Uint(u1, u2, u3, u4) => AttributeValue::Vec4Uint(u1, u2, u3, u4),
            AttributeValue::Any(_) => panic!("Any not supported"),
        }
    }
}

impl TemplateValue for OwnedTemplateValue {
    fn allocate<'b>(&self, bump: &'b Bump) -> AttributeValue<'b> {
        match self.clone() {
            OwnedTemplateValue::Text(txt) => AttributeValue::Text(bump.alloc(txt)),
            OwnedTemplateValue::Bytes(bytes) => AttributeValue::Bytes(bump.alloc(bytes)),
            OwnedTemplateValue::Float32(f) => AttributeValue::Float32(f),
            OwnedTemplateValue::Float64(f) => AttributeValue::Float64(f),
            OwnedTemplateValue::Int32(i) => AttributeValue::Int32(i),
            OwnedTemplateValue::Int64(i) => AttributeValue::Int64(i),
            OwnedTemplateValue::Uint32(u) => AttributeValue::Uint32(u),
            OwnedTemplateValue::Uint64(u) => AttributeValue::Uint64(u),
            OwnedTemplateValue::Bool(b) => AttributeValue::Bool(b),
            OwnedTemplateValue::Vec3Float(f1, f2, f3) => AttributeValue::Vec3Float(f1, f2, f3),
            OwnedTemplateValue::Vec3Int(i1, i2, i3) => AttributeValue::Vec3Int(i1, i2, i3),
            OwnedTemplateValue::Vec3Uint(u1, u2, u3) => AttributeValue::Vec3Uint(u1, u2, u3),
            OwnedTemplateValue::Vec4Float(f1, f2, f3, f4) => {
                AttributeValue::Vec4Float(f1, f2, f3, f4)
            }
            OwnedTemplateValue::Vec4Int(i1, i2, i3, i4) => AttributeValue::Vec4Int(i1, i2, i3, i4),
            OwnedTemplateValue::Vec4Uint(u1, u2, u3, u4) => {
                AttributeValue::Vec4Uint(u1, u2, u3, u4)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum TemplateNodeType<Attributes, V, Children, Listeners, TextSegments, Text, Fragment>
where
    Fragment: AsRef<[TemplateNodeId]>,
    Attributes: AsRef<[TemplateAttribute<V>]>,
    Children: AsRef<[TemplateNodeId]>,
    Listeners: AsRef<[usize]>,
    V: TemplateValue,
    TextSegments: AsRef<[TextTemplateSegment<Text>]>,
    Text: AsRef<str>,
{
    Element(TemplateElement<Attributes, V, Children, Listeners>),
    Text(TextTemplate<TextSegments, Text>),
    Fragment(Fragment),
    /// The index in the dynamic node array this node should be replaced with
    DynamicNode(usize),
}

#[derive(Debug)]
pub(crate) struct TemplateElement<Attributes, V, Children, Listeners>
where
    Attributes: AsRef<[TemplateAttribute<V>]>,
    Children: AsRef<[TemplateNodeId]>,
    Listeners: AsRef<[usize]>,
    V: TemplateValue,
{
    pub(crate) tag: &'static str,
    pub(crate) namespace: Option<&'static str>,
    pub(crate) attributes: Attributes,
    pub(crate) children: Children,
    pub(crate) listeners: Listeners,
    pub(crate) parent: Option<TemplateNodeId>,
    value: PhantomData<V>,
}

#[derive(Debug)]
pub(crate) struct TextTemplate<Segments, Text>
where
    Segments: AsRef<[TextTemplateSegment<Text>]>,
    Text: AsRef<str>,
{
    // this is similar to what ifmt outputs and allows us to only diff the dynamic parts of the text
    pub segments: Segments,
    text: PhantomData<Text>,
}

#[derive(Debug)]
pub(crate) enum TextTemplateSegment<Text>
where
    Text: AsRef<str>,
{
    Static(Text),
    Dynamic(usize),
}

#[derive(Debug, Clone)]
pub(crate) enum OwnedTemplateValue {
    Text(String),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
    Bool(bool),

    Vec3Float(f32, f32, f32),
    Vec3Int(i32, i32, i32),
    Vec3Uint(u32, u32, u32),

    Vec4Float(f32, f32, f32, f32),
    Vec4Int(i32, i32, i32, i32),
    Vec4Uint(u32, u32, u32, u32),

    Bytes(Vec<u8>),
    // TODO: support other types
    // Any(ArbitraryAttributeValue<'a>),
}
