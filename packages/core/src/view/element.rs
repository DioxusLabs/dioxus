//! The typed element view: [`ElementBuilder`] and its tag marker
//! [`ElementTag`].

use std::marker::PhantomData;

use crate::DynamicValues;
use dioxus_core_template::TemplateRawTree;

use super::{DynamicAttributesBuilder, IntoViewChild, View, ViewTemplate};

/// A static element tag marker.
pub trait ElementTag {
    /// The renderer tag name.
    const NAME: &'static str;

    /// The optional renderer namespace.
    const NAMESPACE: Option<&'static str> = None;
}

/// A typed element view.
pub struct ElementBuilder<Tag, Attributes, Children> {
    pub(super) attrs: Attributes,
    pub(super) children: Children,
    pub(super) _tag: PhantomData<Tag>,
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
}

/// An empty typed element (`html::div`) paired with its attribute views and its children.
///
/// `rsx!` builds elements with this struct literal rather than a chain of attribute and
/// [`ElementBuilder::child`] calls: attributes and children are tuples of typed views, so no
/// per-element method instantiation is needed to join them and the element type stays flat.
/// It lowers exactly like the equivalent `ElementBuilder<Tag, Attributes, Children>`.
#[doc(hidden)]
pub struct ElementParts<Element, Attributes, Children>(pub Element, pub Attributes, pub Children);

impl<Tag: ElementTag, Attributes: ViewTemplate, Children: ViewTemplate> ViewTemplate
    for ElementParts<ElementBuilder<Tag, (), ()>, Attributes, Children>
{
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::Element {
        tag: Tag::NAME,
        namespace: Tag::NAMESPACE,
        attrs: Attributes::TEMPLATE_TREE,
        children: Children::TEMPLATE_TREE,
    };
    const HAS_DYNAMIC: bool = Attributes::HAS_DYNAMIC || Children::HAS_DYNAMIC;
}

impl<Tag: ElementTag, Attributes: View, Children: View> View
    for ElementParts<ElementBuilder<Tag, (), ()>, Attributes, Children>
{
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        if Attributes::HAS_DYNAMIC {
            self.1.push(dynamic);
        }
        if Children::HAS_DYNAMIC {
            self.2.push(dynamic);
        }
    }
}

/// The single runtime attribute a generated attribute method appended to an empty element, as a
/// standalone attribute view for an [`ElementParts`] attribute tuple.
#[doc(hidden)]
#[inline(always)]
pub fn element_attribute<Tag>(
    element: ElementBuilder<Tag, ((), DynamicAttributesBuilder), ()>,
) -> DynamicAttributesBuilder {
    element.attrs.1
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
    const HAS_DYNAMIC: bool = Attributes::HAS_DYNAMIC || Children::HAS_DYNAMIC;
}

impl<Tag: ElementTag, Attributes: View, Children: View> View
    for ElementBuilder<Tag, Attributes, Children>
{
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        if Attributes::HAS_DYNAMIC {
            self.attrs.push(dynamic);
        }
        if Children::HAS_DYNAMIC {
            self.children.push(dynamic);
        }
    }
}
