//! Typed, const-driven view builders.
//!
//! This module mirrors the template-v2 builder model: each view type contributes
//! const raw template structure, and the composed type is promoted to a static
//! [`Template`] through [`ViewTemplate`].

use std::marker::PhantomData;
#[cfg(debug_assertions)]
use std::sync::OnceLock;

use crate::{
    Attribute, DynamicNode, DynamicValue, HasAttributes, IntoAttributeValue, IntoDynNode,
    RenderedView, Template, VComponent, VNode,
    nodes::IntoVNode,
    template::{
        TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
        TemplateRawTree, TemplateStorage,
    },
};

/// A type that contributes static template structure.
#[doc(hidden)]
pub trait ViewTemplate {
    /// The raw template-v2-style tree for this view type.
    const TEMPLATE_TREE: &'static TemplateRawTree;
}

trait StaticViewTemplate: ViewTemplate {
    /// The static template for this view type.
    #[cfg(not(debug_assertions))]
    const TEMPLATE: &'static Template;

    /// Build the static template for this view type.
    #[cfg(debug_assertions)]
    fn build_template() -> Template;

    /// Return the template for this view type from a call-site cache.
    #[cfg(debug_assertions)]
    fn template_from_cell(cell: &'static OnceLock<Template>) -> &'static Template;
}

impl<T: ViewTemplate> StaticViewTemplate for T {
    #[cfg(not(debug_assertions))]
    const TEMPLATE: &'static Template = &TemplateStorage::<
        TEMPLATE_STORAGE_OPS_CAP,
        TEMPLATE_STORAGE_STRING_CAP,
        TEMPLATE_STORAGE_DYNAMIC_CAP,
    >::build_from_tree(T::TEMPLATE_TREE)
    .as_template();

    #[cfg(debug_assertions)]
    #[inline]
    fn build_template() -> Template {
        TemplateStorage::<
            TEMPLATE_STORAGE_OPS_CAP,
            TEMPLATE_STORAGE_STRING_CAP,
            TEMPLATE_STORAGE_DYNAMIC_CAP,
        >::build_from_tree(T::TEMPLATE_TREE)
        .into_leaked_template()
    }

    #[cfg(debug_assertions)]
    #[inline]
    fn template_from_cell(cell: &'static OnceLock<Template>) -> &'static Template {
        cell.get_or_init(Self::build_template)
    }
}

impl ViewTemplate for () {
    const TEMPLATE_TREE: &'static TemplateRawTree = TemplateRawTree::EMPTY;
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
    pub(crate) fn with_capacity(capacity: usize) -> Self {
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
    pub(crate) fn into_boxed_slice_for_template(self, template: &Template) -> Box<[DynamicValue]> {
        template
            .reorder_dynamic_values_from_document_order(self.values)
            .into_boxed_slice()
    }

    /// Convert this buffer into the rendered view payload expected by [`VNode`].
    #[inline]
    pub(crate) fn into_rendered_view_for_template(
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
}

/// Extension methods for typed views.
pub trait ViewExt: View {
    /// Convert this view into a [`VNode`].
    fn into_vnode(self) -> VNode;
}

impl<V: View> ViewExt for V {
    #[inline]
    fn into_vnode(self) -> VNode {
        into_vnode_with_key(self, None)
    }
}

/// Convert a view into a [`VNode`] using a prepared template.
#[doc(hidden)]
#[inline]
pub fn into_vnode_with_template<V: View>(
    view: V,
    key: Option<String>,
    template: &Template,
) -> VNode {
    let mut dynamic = DynamicViewValues::with_capacity(template.dynamic_value_count());
    view.push(&mut dynamic);
    VNode::new_with_rendered_view(
        *template,
        dynamic.into_rendered_view_for_template(key, template),
    )
}

/// Convert a view into a keyed [`VNode`].
#[doc(hidden)]
#[inline]
pub fn into_vnode_with_key<V: View>(view: V, key: Option<String>) -> VNode {
    #[cfg(debug_assertions)]
    {
        into_vnode_with_template(view, key, &<V as StaticViewTemplate>::build_template())
    }

    #[cfg(not(debug_assertions))]
    {
        into_vnode_with_template(view, key, <V as StaticViewTemplate>::TEMPLATE)
    }
}

/// Convert a view into a keyed [`VNode`] using a lazily initialized template cache.
#[cfg(debug_assertions)]
#[doc(hidden)]
#[inline]
pub fn into_vnode_with_key_and_template_cell<V: View>(
    view: V,
    key: Option<String>,
    template_cell: &'static OnceLock<Template>,
) -> VNode {
    into_vnode_with_template(
        view,
        key,
        <V as StaticViewTemplate>::template_from_cell(template_cell),
    )
}

impl View for () {}

impl ViewTemplate for VComponent {
    const TEMPLATE_TREE: &'static TemplateRawTree = TemplateRawTree::DYNAMIC_NODE;
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
        ViewExt::into_vnode(self)
    }
}

macro_rules! impl_tuple_views {
    (($($name:ident $value:ident,)*) ;) => {};
    (($($name:ident $value:ident,)*) ; $next_name:ident $next_value:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl $($name $value,)* $next_name $next_value,);
        impl_tuple_views!(($($name $value,)* $next_name $next_value,) ; $($rest)*);
    };
    (@impl $first_name:ident $first_value:ident,) => {
        impl<$first_name: ViewTemplate> ViewTemplate for ($first_name,) {
            const TEMPLATE_TREE: &'static TemplateRawTree = $first_name::TEMPLATE_TREE;
        }

        impl<$first_name: View> View for ($first_name,) {
            #[inline]
            fn push(self, dynamic: &mut DynamicViewValues) {
                let ($first_value,) = self;
                $first_value.push(dynamic);
            }
        }
    };
    (@impl $first_name:ident $first_value:ident, $($name:ident $value:ident,)+) => {
        impl<$first_name: ViewTemplate, $($name: ViewTemplate),*> ViewTemplate for ($first_name, $($name,)*) {
            const TEMPLATE_TREE: &'static TemplateRawTree =
                &TemplateRawTree::Sequence(&[$first_name::TEMPLATE_TREE, $($name::TEMPLATE_TREE,)*]);
        }

        impl<$first_name: View, $($name: View),*> View for ($first_name, $($name,)*) {
            #[inline]
            fn push(self, dynamic: &mut DynamicViewValues) {
                let ($first_value, $($value,)*) = self;
                $first_value.push(dynamic);
                $($value.push(dynamic);)*
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

    /// Replace the children with an already-normalized typed view tuple.
    #[doc(hidden)]
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

/// Marker for child values that are already typed views.
#[doc(hidden)]
pub struct ViewChildMarker;

pub(crate) mod dynamic_node {
    use std::marker::PhantomData;

    use crate::{IntoDynNode, template::TemplateRawTree};

    use super::{DynamicViewValues, View, ViewTemplate};

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
        const TEMPLATE_TREE: &'static TemplateRawTree = TemplateRawTree::DYNAMIC_NODE;
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
pub struct StaticAttributeBuilder<Descriptor, Value = Descriptor>(PhantomData<(Descriptor, Value)>);

/// Create a static attribute view for an attribute marker.
#[doc(hidden)]
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
    const TEMPLATE_TREE: &'static TemplateRawTree = TemplateRawTree::DYNAMIC_ATTR;
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
    type Output =
        ElementBuilder<Tag, (Attributes, StaticAttributeBuilder<Descriptor, Value>), Children>;

    #[inline]
    fn append_to(self, target: ElementBuilder<Tag, Attributes, Children>) -> Self::Output {
        target.attribute(StaticAttributeBuilder(PhantomData))
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
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::StaticText(T::TEXT);
}

impl<T: StaticText> View for StaticTextBuilder<T> {}

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
