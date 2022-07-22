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
    pub nodes: Nodes,
    text_inner: PhantomData<TextInner>,
    pub text: TextOuter,
    pub attributes: AttributesOuter,
    pub volitile_attributes: AttributesInner,
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
    pub fn new(
        nodes: Nodes,
        text: TextOuter,
        attributes: AttributesOuter,
        volitile_attributes: AttributesInner,
    ) -> Self {
        DynamicNodeMapping {
            nodes,
            text_inner: PhantomData,
            text,
            attributes,
            volitile_attributes,
        }
    }

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
