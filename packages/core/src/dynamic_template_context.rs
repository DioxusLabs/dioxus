use std::{marker::PhantomData, ops::Deref};

use once_cell::sync::Lazy;

use crate::{
    template::{TemplateNodeId, TextTemplateSegment},
    AttributeValue, Listener, VNode,
};

/// A lazily initailized vector
#[derive(Debug, Clone, Copy)]
pub struct LazyStaticVec<T: 'static>(pub &'static Lazy<Vec<T>>);

impl<T: 'static> AsRef<[T]> for LazyStaticVec<T> {
    fn as_ref(&self) -> &[T] {
        let v: &Vec<_> = self.0.deref();
        v.as_ref()
    }
}

impl<T> PartialEq for LazyStaticVec<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

// Stores what nodes depend on specific dynamic parts of the template to allow the diffing algorithm to jump to that part of the template instead of travering it
// This makes adding constant template nodes add no additional cost to diffing.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct DynamicNodeMapping<
    Nodes,
    TextOuter,
    TextInner,
    AttributesOuter,
    AttributesInner,
    Volatile,
    Listeners,
> where
    Nodes: AsRef<[Option<TemplateNodeId>]>,
    TextOuter: AsRef<[TextInner]>,
    TextInner: AsRef<[TemplateNodeId]>,
    AttributesOuter: AsRef<[AttributesInner]>,
    AttributesInner: AsRef<[(TemplateNodeId, usize)]>,
    Volatile: AsRef<[(TemplateNodeId, usize)]>,
    Listeners: AsRef<[TemplateNodeId]>,
{
    pub nodes: Nodes,
    text_inner: PhantomData<TextInner>,
    pub text: TextOuter,
    pub attributes: AttributesOuter,
    attributes_inner: PhantomData<AttributesInner>,
    pub volatile_attributes: Volatile,
    pub nodes_with_listeners: Listeners,
}

impl<Nodes, TextOuter, TextInner, AttributesOuter, AttributesInner, Volatile, Listeners>
    DynamicNodeMapping<
        Nodes,
        TextOuter,
        TextInner,
        AttributesOuter,
        AttributesInner,
        Volatile,
        Listeners,
    >
where
    Nodes: AsRef<[Option<TemplateNodeId>]>,
    TextOuter: AsRef<[TextInner]>,
    TextInner: AsRef<[TemplateNodeId]>,
    AttributesOuter: AsRef<[AttributesInner]>,
    AttributesInner: AsRef<[(TemplateNodeId, usize)]>,
    Volatile: AsRef<[(TemplateNodeId, usize)]>,
    Listeners: AsRef<[TemplateNodeId]>,
{
    pub const fn new(
        nodes: Nodes,
        text: TextOuter,
        attributes: AttributesOuter,
        volatile_attributes: Volatile,
        listeners: Listeners,
    ) -> Self {
        DynamicNodeMapping {
            nodes,
            text_inner: PhantomData,
            text,
            attributes,
            attributes_inner: PhantomData,
            volatile_attributes,
            nodes_with_listeners: listeners,
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
            .chain(self.nodes_with_listeners.as_ref().iter().copied())
    }
}

/// A dynamic node mapping that is stack allocated
pub type StaticDynamicNodeMapping = DynamicNodeMapping<
    &'static [Option<TemplateNodeId>],
    &'static [&'static [TemplateNodeId]],
    &'static [TemplateNodeId],
    &'static [&'static [(TemplateNodeId, usize)]],
    &'static [(TemplateNodeId, usize)],
    // volatile attribute information is available at compile time, but there is no way for the macro to generate it, so we initialize it lazily instead
    LazyStaticVec<(TemplateNodeId, usize)>,
    &'static [TemplateNodeId],
>;

#[cfg(feature = "hot-reload")]
/// A dynamic node mapping that is heap allocated
pub type OwnedDynamicNodeMapping = DynamicNodeMapping<
    Vec<Option<TemplateNodeId>>,
    Vec<Vec<TemplateNodeId>>,
    Vec<TemplateNodeId>,
    Vec<Vec<(TemplateNodeId, usize)>>,
    Vec<(TemplateNodeId, usize)>,
    Vec<(TemplateNodeId, usize)>,
    Vec<TemplateNodeId>,
>;

/// The dynamic parts used to saturate a template durring runtime
pub struct TemplateContext<'b> {
    /// The dynamic nodes
    pub nodes: &'b [VNode<'b>],
    /// The dynamic text
    pub text_segments: &'b [&'b str],
    /// The dynamic attributes
    pub attributes: &'b [AttributeValue<'b>],
    /// The dynamic attributes
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
