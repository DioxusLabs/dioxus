//! Typed attribute views: static and dynamic attribute builders plus the
//! traits that append attribute values onto a builder target.

use std::marker::PhantomData;

use crate::{Attribute, DynamicValues, HasAttributes, IntoAttributeValue};
use dioxus_core_template::TemplateRawTree;

use super::{ElementBuilder, View, ViewTemplate};

/// Static metadata for a generated attribute builder method.
pub trait AttributeDescriptor {
    /// Attribute name.
    const NAME: &'static str;

    /// Attribute namespace.
    const NAMESPACE: Option<&'static str> = None;

    /// Whether this dynamic attribute should always be written.
    const VOLATILE: bool = false;
}

/// A static attribute view.
pub struct StaticAttributeBuilder<Descriptor, Value = Descriptor>(PhantomData<(Descriptor, Value)>);

/// Create a static attribute view for an attribute marker.
#[inline]
pub const fn static_attribute<A: AttributeDescriptor + StaticAttributeValue>()
-> StaticAttributeBuilder<A> {
    StaticAttributeBuilder(PhantomData)
}

impl<Descriptor, Value> ViewTemplate for StaticAttributeBuilder<Descriptor, Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::StaticAttr {
        name: Descriptor::NAME,
        value: Value::VALUE,
        namespace: Descriptor::NAMESPACE,
    };
    const HAS_DYNAMIC: bool = false;
}

impl<Descriptor, Value> View for StaticAttributeBuilder<Descriptor, Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
}

/// A marker for one static attribute value.
pub trait StaticAttributeValue {
    /// Attribute value.
    const VALUE: &'static str;
}

/// A static attribute value that can be passed to typed attribute methods.
pub struct StaticAttributeValueBuilder<V>(#[doc(hidden)] pub PhantomData<V>);

/// Marker for static attribute values passed to typed attribute methods.
#[doc(hidden)]
pub struct StaticAttributeValueBuilderMarker;

/// An element whose generated attribute methods build zero-sized [`StaticAttributeBuilder`]s
/// instead of appending to the element.
///
/// `Static(html::div).class(StaticAttributeValueBuilder::<S>(PhantomData))` is the
/// `StaticAttributeBuilder<ClassDescriptor, S>` view of that one attribute, so `rsx!` can lower
/// an element's literal attributes as one flat tuple next to the tag rather than a chain of
/// builder calls that grows the element type at every step.
#[doc(hidden)]
pub struct Static<Element>(pub Element);

/// A value generated attribute methods can be called on: an element view, a [`Static`] element
/// or a runtime attribute list such as a `#[props(extends = ..)]` builder.
pub trait AttributeTarget: Sized {}

/// A dynamic attribute slot whose values were pushed ahead of the view.
///
/// `rsx!` evaluates every dynamic attribute into the body's [`DynamicValues`] before the typed
/// view (see [`push_dyn_attrs`] and [`push_element_attrs`]) and marks its template position with
/// this zero-sized view, so the view itself never carries runtime attribute values and
/// [`View::push`] is never instantiated for it.
#[doc(hidden)]
pub struct DynamicAttributeSlot;

impl ViewTemplate for DynamicAttributeSlot {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicAttr;
    const HAS_DYNAMIC: bool = false;
}

impl View for DynamicAttributeSlot {}

/// Push an already boxed attribute list onto `dynamic`, filling the next
/// [`DynamicAttributeSlot`] of the body's template in order.
#[doc(hidden)]
#[inline]
pub fn push_dyn_attrs(dynamic: &mut DynamicValues, attrs: Box<[Attribute]>) {
    dynamic.push_attrs(attrs);
}

/// Push the single attribute a generated attribute method appended to an empty element onto
/// `dynamic`, filling the next [`DynamicAttributeSlot`] of the body's template in order.
///
/// `rsx!` calls this with `html::div.class(value)` so the attribute method still resolves the
/// attribute's name, namespace and value conversion; only the tag marker is generic here.
#[doc(hidden)]
#[inline]
pub fn push_element_attrs<Tag>(
    dynamic: &mut DynamicValues,
    element: ElementBuilder<Tag, ((), DynamicAttributesBuilder), ()>,
) {
    dynamic.push_attrs(super::element_attribute(element).attrs);
}

/// A dynamic attribute slot.
pub struct DynamicAttributesBuilder {
    attrs: Box<[Attribute]>,
}

/// Create a dynamic attribute slot from an already boxed attribute list.
#[inline]
#[doc(hidden)]
pub fn dynamic_attributes_builder(attrs: Box<[Attribute]>) -> DynamicAttributesBuilder {
    DynamicAttributesBuilder { attrs }
}

impl ViewTemplate for DynamicAttributesBuilder {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicAttr;
}

impl View for DynamicAttributesBuilder {
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_attrs(self.attrs);
    }
}

/// How an [`AttributeTarget`] takes one attribute value from a generated attribute method.
///
/// The target type selects the implementation, so a method call only ever has one candidate to
/// check: runtime targets ([`AppendAttribute`]) convert the value and append the attribute, while
/// a [`Static`] element builds a zero-sized static attribute view.
pub trait AttributeBuilderTarget<Descriptor, Value, Marker>: AttributeTarget
where
    Descriptor: AttributeDescriptor,
{
    /// The value returned by the attribute method.
    type Output;

    /// Take the attribute value.
    fn with_attribute(self, value: Value) -> Self::Output;
}

/// A runtime attribute target: one fully constructed [`Attribute`] can be appended to it.
pub trait AppendAttribute: AttributeTarget {
    /// The target returned after adding the attribute.
    type Output;

    /// Append one fully constructed attribute.
    fn append_attribute(self, attr: Attribute) -> Self::Output;
}

impl<Target, Descriptor, Value, Marker> AttributeBuilderTarget<Descriptor, Value, Marker> for Target
where
    Target: AppendAttribute,
    Descriptor: AttributeDescriptor,
    Value: IntoAttributeValue<Marker>,
{
    type Output = <Target as AppendAttribute>::Output;

    #[inline(always)]
    fn with_attribute(self, value: Value) -> Self::Output {
        self.append_attribute(Attribute::new(
            Descriptor::NAME,
            value,
            Descriptor::NAMESPACE,
            Descriptor::VOLATILE,
        ))
    }
}

impl<Target> AttributeTarget for Target where Target: HasAttributes {}

impl<Target> AppendAttribute for Target
where
    Target: HasAttributes,
{
    type Output = Self;

    #[inline(always)]
    fn append_attribute(self, attr: Attribute) -> Self::Output {
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }
}

impl<Tag, Attributes, Children> AttributeTarget for ElementBuilder<Tag, Attributes, Children> {}

impl<Tag, Attributes, Children> AppendAttribute for ElementBuilder<Tag, Attributes, Children> {
    type Output = ElementBuilder<Tag, (Attributes, DynamicAttributesBuilder), Children>;

    #[inline(always)]
    fn append_attribute(self, attr: Attribute) -> Self::Output {
        ElementBuilder {
            attrs: (
                self.attrs,
                DynamicAttributesBuilder {
                    attrs: Box::new([attr]),
                },
            ),
            children: self.children,
            _tag: PhantomData,
        }
    }
}

impl AttributeTarget for Vec<Attribute> {}

impl AppendAttribute for Vec<Attribute> {
    type Output = Self;

    #[inline(always)]
    fn append_attribute(mut self, attr: Attribute) -> Self::Output {
        self.push(attr);
        self
    }
}

impl<Element> AttributeTarget for Static<Element> {}

impl<Element, Descriptor, Value>
    AttributeBuilderTarget<
        Descriptor,
        StaticAttributeValueBuilder<Value>,
        StaticAttributeValueBuilderMarker,
    > for Static<Element>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    type Output = StaticAttributeBuilder<Descriptor, Value>;

    #[inline(always)]
    fn with_attribute(self, _value: StaticAttributeValueBuilder<Value>) -> Self::Output {
        StaticAttributeBuilder(PhantomData)
    }
}
