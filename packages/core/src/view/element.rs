//! The typed element view: [`ElementBuilder`] and its tag marker
//! [`ElementTag`].

use std::marker::PhantomData;

use crate::DynamicValues;
use dioxus_core_template::TemplateRawTree;

use super::{IntoViewChild, View, ViewTemplate};

/// A static element tag marker.
pub trait ElementTag {
    /// The renderer tag name.
    const NAME: &'static str;

    /// The optional renderer namespace.
    const NAMESPACE: Option<&'static str> = None;
}

/// A typed element view.
pub struct ElementBuilder<Tag, Attributes, Children> {
    attrs: Attributes,
    children: Children,
    _tag: PhantomData<Tag>,
}

/// Create an empty typed element for a tag marker.
#[inline]
pub const fn element_builder<Tag>() -> ElementBuilder<Tag, (), ()> {
    ElementBuilder {
        attrs: (),
        children: (),
        _tag: PhantomData,
    }
}

impl<Tag, Attributes, Children> ElementBuilder<Tag, Attributes, Children> {
    /// Append one attribute view.
    #[inline]
    pub fn attribute<AttributeView>(
        self,
        attr: AttributeView,
    ) -> ElementBuilder<Tag, (Attributes, AttributeView), Children> {
        ElementBuilder {
            attrs: (self.attrs, attr),
            children: self.children,
            _tag: PhantomData,
        }
    }

    /// Append one child.
    #[inline]
    pub fn child<Child, Marker>(
        self,
        child: Child,
    ) -> ElementBuilder<Tag, Attributes, (Children, <Child as IntoViewChild<Marker>>::Output)>
    where
        Child: IntoViewChild<Marker>,
    {
        ElementBuilder {
            attrs: self.attrs,
            children: (self.children, child.into_child()),
            _tag: PhantomData,
        }
    }

    /// Replace the children with an already-normalized typed view tuple.
    #[inline]
    pub fn with_children<NewChildren>(
        self,
        children: NewChildren,
    ) -> ElementBuilder<Tag, Attributes, NewChildren> {
        ElementBuilder {
            attrs: self.attrs,
            children,
            _tag: PhantomData,
        }
    }
}

impl<Tag: ElementTag, Attributes: ViewTemplate, Children: ViewTemplate> ViewTemplate
    for ElementBuilder<Tag, Attributes, Children>
{
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::Element {
        tag: Tag::NAME,
        namespace: Tag::NAMESPACE,
        attrs: Attributes::TEMPLATE_TREE,
        children: Children::TEMPLATE_TREE,
    };
}

impl<Tag: ElementTag, Attributes: View, Children: View> View
    for ElementBuilder<Tag, Attributes, Children>
{
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        self.attrs.push(dynamic);
        self.children.push(dynamic);
    }
}
