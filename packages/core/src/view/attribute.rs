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
pub struct StaticAttributeValueBuilder<V>(PhantomData<V>);

/// Create a static attribute value from a marker type.
#[inline]
pub const fn static_attribute_value<V: StaticAttributeValue>() -> StaticAttributeValueBuilder<V> {
    StaticAttributeValueBuilder(PhantomData)
}

/// Marker for static attribute values passed to typed attribute methods.
#[doc(hidden)]
pub struct StaticAttributeValueBuilderMarker;

/// A value that can be appended by a generated attribute builder method.
pub trait IntoAttributeBuilderValue<Target, Descriptor, Marker>
where
    Target: AttributeBuilderTarget,
    Descriptor: AttributeDescriptor,
{
    /// The target returned after appending this attribute value.
    type Output;

    /// Append this value to the target.
    fn append_to(self, target: Target) -> Self::Output;
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

/// Create a dynamic attribute slot.
#[inline]
pub fn dynamic_attribute<T>(
    name: &'static str,
    value: impl IntoAttributeValue<T>,
    namespace: Option<&'static str>,
    volatile: bool,
) -> DynamicAttributesBuilder {
    DynamicAttributesBuilder {
        attrs: Box::new([Attribute::new(name, value, namespace, volatile)]),
    }
}

impl ViewTemplate for DynamicAttributesBuilder {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicAttr;
}

impl View for DynamicAttributesBuilder {
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_attrs(self.attrs);
    }
}

/// A builder target that can accept one attribute.
pub trait AttributeBuilderTarget: Sized {
    /// The target returned after adding the attribute.
    type Output;

    /// Append one fully constructed attribute.
    fn append_attribute(self, attr: Attribute) -> Self::Output;
}

impl<Target> AttributeBuilderTarget for Target
where
    Target: HasAttributes,
{
    type Output = Self;

    #[inline]
    fn append_attribute(self, attr: Attribute) -> Self::Output {
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }
}

impl<Tag, Attributes, Children> AttributeBuilderTarget
    for ElementBuilder<Tag, Attributes, Children>
{
    type Output = ElementBuilder<Tag, (Attributes, DynamicAttributesBuilder), Children>;

    #[inline]
    fn append_attribute(self, attr: Attribute) -> Self::Output {
        self.attribute(dynamic_attributes_builder(Box::new([attr])))
    }
}

impl AttributeBuilderTarget for Vec<Attribute> {
    type Output = Self;

    #[inline]
    fn append_attribute(mut self, attr: Attribute) -> Self::Output {
        self.push(attr);
        self
    }
}

impl<Target, Descriptor, Marker, Value> IntoAttributeBuilderValue<Target, Descriptor, Marker>
    for Value
where
    Target: AttributeBuilderTarget,
    Descriptor: AttributeDescriptor,
    Value: IntoAttributeValue<Marker>,
{
    type Output = <Target as AttributeBuilderTarget>::Output;

    #[inline]
    fn append_to(self, target: Target) -> Self::Output {
        AttributeBuilderTarget::append_attribute(
            target,
            Attribute::new(
                Descriptor::NAME,
                self,
                Descriptor::NAMESPACE,
                Descriptor::VOLATILE,
            ),
        )
    }
}

impl<Tag, Attributes, Children, Descriptor, Value>
    IntoAttributeBuilderValue<
        ElementBuilder<Tag, Attributes, Children>,
        Descriptor,
        StaticAttributeValueBuilderMarker,
    > for StaticAttributeValueBuilder<Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    type Output =
        ElementBuilder<Tag, (Attributes, StaticAttributeBuilder<Descriptor, Value>), Children>;

    #[inline]
    fn append_to(self, target: ElementBuilder<Tag, Attributes, Children>) -> Self::Output {
        target.attribute(StaticAttributeBuilder(PhantomData))
    }
}

/// Declare a static attribute value for typed attribute methods.
#[macro_export]
macro_rules! static_attribute_value {
    ($value:literal) => {{
        struct StaticAttributeValueMarker;

        impl $crate::view::StaticAttributeValue for StaticAttributeValueMarker {
            const VALUE: &'static str = $value;
        }

        $crate::view::static_attribute_value::<StaticAttributeValueMarker>()
    }};
}
