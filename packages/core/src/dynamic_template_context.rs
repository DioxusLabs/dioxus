use std::marker::PhantomData;

use crate::{
    templete::{TemplateNodeId, TextTemplateSegment},
    AttributeValue, Listener, VNode,
};

// stores what nodes depend on specific dynamic parts of the template to allow the diffing algorithm to jump to that part of the template instead of travering it
#[derive(Debug)]
pub struct DynamicNodeMapping<Nodes, TextOuter, TextInner, AttributesOuter, AttributesInner>
where
    Nodes: AsRef<[Option<TemplateNodeId>]>,
    TextOuter: AsRef<[TextInner]>,
    TextInner: AsRef<[TemplateNodeId]>,
    AttributesOuter: AsRef<[AttributesInner]>,
    AttributesInner: AsRef<[(TemplateNodeId, usize)]>,
{
    nodes: Nodes,
    text_inner: PhantomData<TextInner>,
    text: TextOuter,
    attributes_inner: PhantomData<AttributesInner>,
    attributes: AttributesOuter,
}

impl<Nodes, TextOuter, TextInner, AttributesOuter, AttributesInner>
    DynamicNodeMapping<Nodes, TextOuter, TextInner, AttributesOuter, AttributesInner>
where
    Nodes: AsRef<[Option<TemplateNodeId>]>,
    TextOuter: AsRef<[TextInner]>,
    TextInner: AsRef<[TemplateNodeId]>,
    AttributesOuter: AsRef<[AttributesInner]>,
    AttributesInner: AsRef<[(TemplateNodeId, usize)]>,
{
    pub(crate) fn all_dynamic<'a>(&'a self) -> impl Iterator<Item = TemplateNodeId> + 'a {
        self.nodes
            .as_ref()
            .iter()
            .filter_map(|o| o.as_ref())
            .chain(
                self.text
                    .as_ref()
                    .iter()
                    .map(|ids| ids.as_ref().iter())
                    .flatten(),
            )
            .copied()
            .chain(
                self.attributes
                    .as_ref()
                    .iter()
                    .map(|ids| ids.as_ref().iter())
                    .flatten()
                    .map(|dynamic| dynamic.0),
            )
    }
}

/// A dynamic node mapping that is stack allocated
pub type StaticDynamicNodeMapping = DynamicNodeMapping<
    &'static [Option<TemplateNodeId>],
    &'static [&'static [TemplateNodeId]],
    &'static [TemplateNodeId],
    &'static [&'static [(TemplateNodeId, usize)]],
    &'static [(TemplateNodeId, usize)],
>;

/// A dynamic node mapping that is heap allocated
pub type OwnedDynamicNodeMapping = DynamicNodeMapping<
    Vec<Option<TemplateNodeId>>,
    Vec<Vec<TemplateNodeId>>,
    Vec<TemplateNodeId>,
    Vec<Vec<(TemplateNodeId, usize)>>,
    Vec<(TemplateNodeId, usize)>,
>;

/// A dynamic node mapping that is either &'static or owned
#[derive(Debug)]
pub(crate) enum AnyDynamicNodeMapping {
    Static(StaticDynamicNodeMapping),
    Owned(OwnedDynamicNodeMapping),
}

impl AnyDynamicNodeMapping {
    pub(crate) fn all_dynamic<'a>(&'a self) -> Box<dyn Iterator<Item = TemplateNodeId> + 'a> {
        match self {
            AnyDynamicNodeMapping::Static(mapping) => Box::new(mapping.all_dynamic()),
            AnyDynamicNodeMapping::Owned(mapping) => Box::new(mapping.all_dynamic()),
        }
    }

    pub(crate) fn get_dynamic_nodes_for_node_index(&self, idx: usize) -> Option<TemplateNodeId> {
        match self {
            AnyDynamicNodeMapping::Static(mapping) => mapping.nodes[idx],
            AnyDynamicNodeMapping::Owned(mapping) => mapping.nodes[idx],
        }
    }

    pub(crate) fn get_dynamic_nodes_for_text_index<'a>(
        &'a self,
        idx: usize,
    ) -> &'a [TemplateNodeId] {
        match self {
            AnyDynamicNodeMapping::Static(mapping) => mapping.text[idx].as_ref(),
            AnyDynamicNodeMapping::Owned(mapping) => mapping.text[idx].as_ref(),
        }
    }

    pub(crate) fn get_dynamic_nodes_for_attribute_index<'a>(
        &'a self,
        idx: usize,
    ) -> &'a [(TemplateNodeId, usize)] {
        match self {
            AnyDynamicNodeMapping::Static(mapping) => mapping.attributes[idx].as_ref(),
            AnyDynamicNodeMapping::Owned(mapping) => mapping.attributes[idx].as_ref(),
        }
    }

    pub(crate) fn to_owned(self) -> Self {
        match self {
            AnyDynamicNodeMapping::Static(mapping) => {
                AnyDynamicNodeMapping::Owned(DynamicNodeMapping {
                    nodes: mapping.nodes.to_vec(),
                    text_inner: PhantomData,
                    text: mapping.text.into_iter().map(|t| t.to_vec()).collect(),
                    attributes_inner: PhantomData,
                    attributes: mapping.attributes.into_iter().map(|t| t.to_vec()).collect(),
                })
            }
            AnyDynamicNodeMapping::Owned(_) => self,
        }
    }
}

pub struct TemplateContext<'b> {
    pub nodes: &'b [VNode<'b>],
    pub text_segments: &'b [&'b str],
    pub attributes: &'b [AttributeValue<'b>],
    // The listeners must not change during the lifetime of the context, use a dynamic node if the listeners change
    pub listeners: &'b [Listener<'b>],
}

impl<'b> TemplateContext<'b> {
    pub(crate) fn resolve_text<TextSegments, Text>(&self, text: &TextSegments) -> String
    where
        TextSegments: AsRef<[TextTemplateSegment<Text>]>,
        Text: AsRef<str>,
    {
        let mut result = String::new();
        for seg in text.as_ref() {
            match seg {
                TextTemplateSegment::Static(s) => result += s.as_ref(),
                TextTemplateSegment::Dynamic(idx) => result += self.text_segments[*idx],
            }
        }
        result
    }

    pub(crate) fn resolve_attribute(&self, idx: usize) -> &'b AttributeValue<'b> {
        &self.attributes[idx]
    }

    pub(crate) fn resolve_listener(&self, idx: usize) -> &'b Listener<'b> {
        &self.listeners[idx]
    }
}
