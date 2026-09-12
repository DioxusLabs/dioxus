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

/// A childless typed element paired with its children.
///
/// `rsx!` attaches children with this struct literal rather than [`ElementBuilder::child`]: the
/// children are always typed views, so no per-element method instantiation is needed to join them,
/// and the pair lowers exactly like the equivalent `ElementBuilder<Tag, Attributes, Children>`.
#[doc(hidden)]
pub struct ElementWithChildren<Element, Children>(pub Element, pub Children);

impl<Tag: ElementTag, Attributes: ViewTemplate, Children: ViewTemplate> ViewTemplate
    for ElementWithChildren<ElementBuilder<Tag, Attributes, ()>, Children>
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
    for ElementWithChildren<ElementBuilder<Tag, Attributes, ()>, Children>
{
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        if Attributes::HAS_DYNAMIC {
            self.0.attrs.push(dynamic);
        }
        if Children::HAS_DYNAMIC {
            self.1.push(dynamic);
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
