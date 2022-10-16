use std::{fmt::Write, marker::PhantomData, ops::Deref};

use once_cell::sync::Lazy;

use crate::{
    template::{TemplateNodeId, TextTemplateSegment},
    AttributeValue, Listener, TextTemplate, VNode,
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

/// Stores what nodes depend on specific dynamic parts of the template to allow the diffing algorithm to jump to that part of the template instead of travering it
/// This makes adding constant template nodes add no additional cost to diffing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// The node that depend on each node in the dynamic template
    pub nodes: Nodes,
    text_inner: PhantomData<TextInner>,
    /// The text nodes that depend on each text segment of the dynamic template
    pub text: TextOuter,
    /// The attributes along with the attribute index in the template that depend on each attribute of the dynamic template
    pub attributes: AttributesOuter,
    attributes_inner: PhantomData<AttributesInner>,
    /// The attributes that are marked as volatile in the template
    pub volatile_attributes: Volatile,
    /// The listeners that depend on each listener of the dynamic template
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
    /// Creates a new dynamic node mapping
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

#[cfg(any(feature = "hot-reload", debug_assertions))]
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
    /// A optional key for diffing
    pub key: Option<&'b str>,
}

impl<'b> TemplateContext<'b> {
    /// Resolve text segments to a string
    pub fn resolve_text<TextSegments, Text>(
        &self,
        text: &TextTemplate<TextSegments, Text>,
    ) -> String
    where
        TextSegments: AsRef<[TextTemplateSegment<Text>]>,
        Text: AsRef<str>,
    {
        let mut result = String::with_capacity(text.min_size);
        self.resolve_text_into(text, &mut result);
        result
    }

    /// Resolve text and writes the result
    pub fn resolve_text_into<TextSegments, Text>(
        &self,
        text: &TextTemplate<TextSegments, Text>,
        result: &mut impl Write,
    ) where
        TextSegments: AsRef<[TextTemplateSegment<Text>]>,
        Text: AsRef<str>,
    {
        for seg in text.segments.as_ref() {
            match seg {
                TextTemplateSegment::Static(s) => {
                    let _ = result.write_str(s.as_ref());
                }
                TextTemplateSegment::Dynamic(idx) => {
                    let _ = result.write_str(self.text_segments[*idx]);
                }
            }
        }
    }

    /// Resolve an attribute value
    pub fn resolve_attribute(&self, idx: usize) -> &'b AttributeValue<'b> {
        &self.attributes[idx]
    }

    /// Resolve a listener
    pub fn resolve_listener(&self, idx: usize) -> &'b Listener<'b> {
        &self.listeners[idx]
    }

    /// Resolve a node
    pub fn resolve_node(&self, idx: usize) -> &'b VNode<'b> {
        &self.nodes[idx]
    }
}
