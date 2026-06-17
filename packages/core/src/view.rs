//! Typed, const-driven view builders.
//!
//! This module mirrors the template-v2 builder model: each view type contributes
//! a const raw template tape, and the composed type is promoted to a static
//! [`Template`] through [`ViewTemplate`].

use std::marker::PhantomData;

use dioxus_const_vec::ConstVec;
use dioxus_core_template::VIEW_TEMPLATE_TAPE_CAP;

use crate::{
    Attribute, DynamicNode, DynamicValue, HasAttributes, IntoAttributeValue, IntoDynNode,
    RenderedView, Template, VComponent, VNode,
    nodes::IntoVNode,
    template::{
        TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
        TemplateRawOp, TemplateStorage,
    },
};

/// A const template-v2-style raw operation tape.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ViewTemplateTape {
    ops: ConstVec<TemplateRawOp, VIEW_TEMPLATE_TAPE_CAP>,
}

impl ViewTemplateTape {
    /// Create an empty raw tape.
    pub(crate) const fn new() -> Self {
        Self {
            ops: ConstVec::new_with_max_size(),
        }
    }

    /// Create a raw tape with one raw template operation.
    pub(crate) const fn single(op: TemplateRawOp) -> Self {
        let mut raw = Self::new();
        raw.push(op);
        raw
    }

    /// Push one raw template operation.
    pub(crate) const fn push(&mut self, op: TemplateRawOp) {
        self.ops.push(op);
    }

    /// Append another raw tape.
    pub(crate) const fn concat(&mut self, other: &ViewTemplateTape) {
        let mut index = 0;
        while index < other.ops.len() {
            self.ops.push(other.ops.at(index));
            index += 1;
        }
    }

    /// Borrow the tape as a static slice during const promotion.
    pub(crate) const fn as_slice(&self) -> &[TemplateRawOp] {
        self.ops.as_slice()
    }
}

impl Default for ViewTemplateTape {
    fn default() -> Self {
        Self::new()
    }
}

/// A type that contributes static template structure.
#[doc(hidden)]
pub trait ViewTemplate {
    /// The raw template-v2-style tape for this view type.
    const TEMPLATE_TAPE: ViewTemplateTape;

    /// The static template for this view type.
    const TEMPLATE: &'static Template = &TemplateStorage::<
        TEMPLATE_STORAGE_OPS_CAP,
        TEMPLATE_STORAGE_STRING_CAP,
        TEMPLATE_STORAGE_DYNAMIC_CAP,
    >::build(Self::TEMPLATE_TAPE.as_slice())
    .as_template();
}

impl ViewTemplate for () {
    const TEMPLATE_TAPE: ViewTemplateTape = ViewTemplateTape::new();
}

/// Runtime dynamic values collected while consuming a typed view.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct DynamicViewValues {
    values: Vec<DynamicValue>,
}

impl DynamicViewValues {
    /// Create a dynamic-value buffer with known capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
        }
    }

    /// Push a dynamic node slot.
    #[inline]
    pub(crate) fn push_node(&mut self, value: DynamicNode) {
        self.values.push(DynamicValue::Node(value));
    }

    /// Push a dynamic attribute slot.
    #[inline]
    pub(crate) fn push_attrs(&mut self, value: Box<[Attribute]>) {
        self.values.push(DynamicValue::Attrs(value));
    }

    /// Convert this buffer into the boxed slice expected by [`VNode`].
    #[inline]
    pub fn into_boxed_slice_for_template(self, template: &Template) -> Box<[DynamicValue]> {
        template
            .reorder_dynamic_values_from_document_order(self.values)
            .into_boxed_slice()
    }

    /// Convert this buffer into the rendered view payload expected by [`VNode`].
    #[inline]
    pub fn into_rendered_view_for_template(
        self,
        key: Option<String>,
        template: &Template,
    ) -> RenderedView {
        RenderedView::new(key, self.into_boxed_slice_for_template(template))
    }
}

/// A typed view that can collect runtime dynamic values.
pub trait View: ViewTemplate + Sized {
    /// Push runtime dynamic values in template order.
    #[inline]
    fn push(self, _dynamic: &mut DynamicViewValues) {}

    /// Convert this view into a [`VNode`].
    #[inline]
    fn into_vnode(self) -> VNode {
        let mut dynamic = DynamicViewValues::with_capacity(Self::TEMPLATE.dynamic_value_count());
        self.push(&mut dynamic);
        VNode::new_with_rendered_view(
            *Self::TEMPLATE,
            dynamic.into_rendered_view_for_template(None, Self::TEMPLATE),
        )
    }

    /// Attach a root key to this view.
    #[inline]
    fn key(self, key: impl IntoViewKey) -> KeyedViewBuilder<Self> {
        KeyedViewBuilder {
            view: self,
            key: key.into_key(),
        }
    }
}

impl View for () {}

impl ViewTemplate for VComponent {
    const TEMPLATE_TAPE: ViewTemplateTape = ViewTemplateTape::single(TemplateRawOp::DynamicNode);
}

impl View for VComponent {
    #[inline]
    fn push(self, dynamic: &mut DynamicViewValues) {
        dynamic.push_node(DynamicNode::Component(self));
    }
}

impl IntoVNode for VComponent {
    #[inline]
    fn into_vnode(self) -> VNode {
        View::into_vnode(self)
    }
}

macro_rules! impl_tuple_views {
    (($($name:ident $value:ident,)*) ;) => {};
    (($($name:ident $value:ident,)*) ; $next_name:ident $next_value:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl $($name $value,)* $next_name $next_value,);
        impl_tuple_views!(($($name $value,)* $next_name $next_value,) ; $($rest)*);
    };
    (@impl $($name:ident $value:ident,)+) => {
        impl<$($name: ViewTemplate),+> ViewTemplate for ($($name,)+) {
            const TEMPLATE_TAPE: ViewTemplateTape = {
                let mut raw = ViewTemplateTape::new();
                $(raw.concat(&$name::TEMPLATE_TAPE);)+
                raw
            };
        }

        impl<$($name: View),+> View for ($($name,)+) {
            #[inline]
            fn push(self, dynamic: &mut DynamicViewValues) {
                let ($($value,)+) = self;
                $($value.push(dynamic);)+
            }
        }
    };
}

impl_tuple_views! {
    ();
    T00 t00,
    T01 t01,
    T02 t02,
    T03 t03,
    T04 t04,
    T05 t05,
    T06 t06,
    T07 t07,
    T08 t08,
    T09 t09,
    T10 t10,
    T11 t11,
    T12 t12,
    T13 t13,
    T14 t14,
    T15 t15,
    T16 t16,
    T17 t17,
    T18 t18,
    T19 t19,
    T20 t20,
    T21 t21,
    T22 t22,
    T23 t23,
    T24 t24,
    T25 t25,
    T26 t26,
    T27 t27,
    T28 t28,
    T29 t29,
    T30 t30,
    T31 t31,
    T32 t32,
    T33 t33,
    T34 t34,
    T35 t35,
    T36 t36,
    T37 t37,
    T38 t38,
    T39 t39,
    T40 t40,
    T41 t41,
    T42 t42,
    T43 t43,
    T44 t44,
    T45 t45,
    T46 t46,
    T47 t47,
    T48 t48,
    T49 t49,
    T50 t50,
    T51 t51,
    T52 t52,
    T53 t53,
    T54 t54,
    T55 t55,
    T56 t56,
    T57 t57,
    T58 t58,
    T59 t59,
    T60 t60,
    T61 t61,
    T62 t62,
    T63 t63,
}

/// A static element tag marker.
#[doc(hidden)]
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
}

/// Marker for child values that are already typed views.
#[doc(hidden)]
pub struct ViewChildMarker;

pub(crate) mod dynamic_node {
    use std::marker::PhantomData;

    use crate::{IntoDynNode, template::TemplateRawOp};

    use super::{DynamicViewValues, View, ViewTemplate, ViewTemplateTape};

    /// Marker for child values that should become dynamic node slots.
    pub struct DynamicViewChildMarker<Marker>(PhantomData<Marker>);

    /// A dynamic node slot.
    pub struct DynamicNodeBuilder<N, Marker = ()> {
        node: N,
        _marker: PhantomData<Marker>,
    }

    /// Create a dynamic node slot from any [`IntoDynNode`] value.
    #[inline]
    pub(crate) fn dynamic_node_builder<N, Marker>(node: N) -> DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        DynamicNodeBuilder {
            node,
            _marker: PhantomData,
        }
    }

    impl<N, Marker> ViewTemplate for DynamicNodeBuilder<N, Marker> {
        const TEMPLATE_TAPE: ViewTemplateTape =
            ViewTemplateTape::single(TemplateRawOp::DynamicNode);
    }

    impl<N, Marker> View for DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        #[inline]
        fn push(self, dynamic: &mut DynamicViewValues) {
            dynamic.push_node(self.node.into_dyn_node());
        }
    }
}

/// Convert a value passed to [`ElementBuilder::child`] into a typed child view.
#[doc(hidden)]
pub trait IntoViewChild<Marker = ViewChildMarker> {
    /// The typed view contributed by this child.
    type Output: View;

    /// Convert into the child view.
    fn into_child(self) -> Self::Output;
}

impl<V: View> IntoViewChild<ViewChildMarker> for V {
    type Output = V;

    #[inline]
    fn into_child(self) -> Self::Output {
        self
    }
}

impl<N, Marker> IntoViewChild<dynamic_node::DynamicViewChildMarker<Marker>> for N
where
    N: IntoDynNode<Marker>,
{
    type Output = dynamic_node::DynamicNodeBuilder<N, Marker>;

    #[inline]
    fn into_child(self) -> Self::Output {
        dynamic_node::dynamic_node_builder(self)
    }
}

impl<Tag: ElementTag, Attributes: ViewTemplate, Children: ViewTemplate> ViewTemplate
    for ElementBuilder<Tag, Attributes, Children>
{
    const TEMPLATE_TAPE: ViewTemplateTape = {
        let mut raw = ViewTemplateTape::new();
        raw.push(TemplateRawOp::OpenElement {
            tag: Tag::NAME,
            namespace: Tag::NAMESPACE,
        });
        raw.concat(&Attributes::TEMPLATE_TAPE);
        raw.concat(&Children::TEMPLATE_TAPE);
        raw.push(TemplateRawOp::CloseElement);
        raw
    };
}

impl<Tag: ElementTag, Attributes: View, Children: View> View
    for ElementBuilder<Tag, Attributes, Children>
{
    #[inline]
    fn push(self, dynamic: &mut DynamicViewValues) {
        self.attrs.push(dynamic);
        self.children.push(dynamic);
    }
}

/// Static metadata for a generated attribute builder method.
#[doc(hidden)]
pub trait AttributeDescriptor {
    /// Attribute name.
    const NAME: &'static str;

    /// Attribute namespace.
    const NAMESPACE: Option<&'static str> = None;

    /// Whether this dynamic attribute should always be written.
    const VOLATILE: bool = false;
}

/// A static attribute view.
#[doc(hidden)]
pub struct StaticAttributeBuilder<A>(PhantomData<A>);

/// Create a static attribute view for an attribute marker.
#[doc(hidden)]
#[inline]
pub const fn static_attribute<A: AttributeDescriptor + StaticAttributeValue>()
-> StaticAttributeBuilder<A> {
    StaticAttributeBuilder(PhantomData)
}

impl<A: AttributeDescriptor + StaticAttributeValue> ViewTemplate for StaticAttributeBuilder<A> {
    const TEMPLATE_TAPE: ViewTemplateTape = ViewTemplateTape::single(TemplateRawOp::StaticAttr {
        name: A::NAME,
        value: A::VALUE,
        namespace: A::NAMESPACE,
    });
}

impl<A: AttributeDescriptor + StaticAttributeValue> View for StaticAttributeBuilder<A> {}

/// A marker for one static attribute value.
#[doc(hidden)]
pub trait StaticAttributeValue {
    /// Attribute value.
    const VALUE: &'static str;
}

/// A static attribute value that can be passed to generated attribute builder methods.
#[doc(hidden)]
pub struct StaticAttributeValueBuilder<V>(PhantomData<V>);

/// Create a static attribute value from a marker type.
#[doc(hidden)]
#[inline]
pub const fn static_attribute_value<V: StaticAttributeValue>() -> StaticAttributeValueBuilder<V> {
    StaticAttributeValueBuilder(PhantomData)
}

/// A static attribute assembled from a generated descriptor and a static value.
#[doc(hidden)]
pub struct StaticAttributeWithValue<Descriptor, Value>(PhantomData<(Descriptor, Value)>);

impl<Descriptor, Value> AttributeDescriptor for StaticAttributeWithValue<Descriptor, Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    const NAME: &'static str = Descriptor::NAME;
    const NAMESPACE: Option<&'static str> = Descriptor::NAMESPACE;
    const VOLATILE: bool = Descriptor::VOLATILE;
}

impl<Descriptor, Value> StaticAttributeValue for StaticAttributeWithValue<Descriptor, Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    const VALUE: &'static str = Value::VALUE;
}

#[doc(hidden)]
pub struct StaticAttributeValueBuilderMarker;

/// A value that can be appended by a generated attribute builder method.
#[doc(hidden)]
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
#[doc(hidden)]
pub struct DynamicAttributesBuilder {
    attrs: Box<[Attribute]>,
}

/// Create a dynamic attribute slot from an already boxed attribute list.
#[inline]
pub(crate) fn dynamic_attributes_builder(attrs: Box<[Attribute]>) -> DynamicAttributesBuilder {
    DynamicAttributesBuilder { attrs }
}

/// Create a dynamic attribute slot with a single attribute.
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
    const TEMPLATE_TAPE: ViewTemplateTape = ViewTemplateTape::single(TemplateRawOp::DynamicAttr);
}

impl View for DynamicAttributesBuilder {
    #[inline]
    fn push(self, dynamic: &mut DynamicViewValues) {
        dynamic.push_attrs(self.attrs);
    }
}

/// A builder target that can accept one attribute.
#[doc(hidden)]
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
    type Output = ElementBuilder<
        Tag,
        (
            Attributes,
            StaticAttributeBuilder<StaticAttributeWithValue<Descriptor, Value>>,
        ),
        Children,
    >;

    #[inline]
    fn append_to(self, target: ElementBuilder<Tag, Attributes, Children>) -> Self::Output {
        target.attribute(static_attribute::<
            StaticAttributeWithValue<Descriptor, Value>,
        >())
    }
}

/// A marker for one static text node.
#[doc(hidden)]
pub trait StaticText {
    /// Static text value.
    const TEXT: &'static str;
}

/// A static text view.
#[doc(hidden)]
pub struct StaticTextBuilder<T>(PhantomData<T>);

/// Create a static text view for a text marker.
#[doc(hidden)]
#[inline]
pub const fn static_text<T: StaticText>() -> StaticTextBuilder<T> {
    StaticTextBuilder(PhantomData)
}

impl<T: StaticText> ViewTemplate for StaticTextBuilder<T> {
    const TEMPLATE_TAPE: ViewTemplateTape =
        ViewTemplateTape::single(TemplateRawOp::StaticText { value: T::TEXT });
}

impl<T: StaticText> View for StaticTextBuilder<T> {}

/// A typed view with a root key.
#[doc(hidden)]
pub struct KeyedViewBuilder<V> {
    view: V,
    key: Option<String>,
}

/// Convert a value into an optional root key.
#[doc(hidden)]
pub trait IntoViewKey {
    /// Convert this value into an optional key.
    fn into_key(self) -> Option<String>;
}

impl IntoViewKey for String {
    #[inline]
    fn into_key(self) -> Option<String> {
        Some(self)
    }
}

impl IntoViewKey for &str {
    #[inline]
    fn into_key(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoViewKey for Option<String> {
    #[inline]
    fn into_key(self) -> Option<String> {
        self
    }
}

impl<V: ViewTemplate> ViewTemplate for KeyedViewBuilder<V> {
    const TEMPLATE_TAPE: ViewTemplateTape = V::TEMPLATE_TAPE;
}

impl<V: View> View for KeyedViewBuilder<V> {
    #[inline]
    fn push(self, dynamic: &mut DynamicViewValues) {
        self.view.push(dynamic);
    }

    #[inline]
    fn into_vnode(self) -> VNode {
        let key = self.key;
        let mut dynamic = DynamicViewValues::with_capacity(Self::TEMPLATE.dynamic_value_count());
        self.view.push(&mut dynamic);
        VNode::new_with_rendered_view(
            *Self::TEMPLATE,
            dynamic.into_rendered_view_for_template(key, Self::TEMPLATE),
        )
    }
}

/// Declare a static text marker type.
#[macro_export]
macro_rules! static_text {
    ($value:literal) => {{
        struct StaticTextMarker;
        impl $crate::view::StaticText for StaticTextMarker {
            const TEXT: &'static str = $value;
        }

        $crate::view::static_text::<StaticTextMarker>()
    }};
    ($name:ident, $value:literal) => {
        $crate::static_text!(pub struct $name, $value);
    };
    ($vis:vis struct $name:ident, $value:expr) => {
        $vis struct $name;
        impl $crate::view::StaticText for $name {
            const TEXT: &'static str = $value;
        }
    };
}

/// Declare a static attribute value for generated attribute builder methods.
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
