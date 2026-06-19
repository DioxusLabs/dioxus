//! Typed view builders.
//!
//! Most applications use `rsx!`, but the generated HTML constructors can also
//! be used directly. A builder can collect attributes and children, then
//! [`ViewExt::into_vnode`] converts it into a [`VNode`].

use std::marker::PhantomData;

use crate::{
    Attribute, DynamicNode, DynamicValues, HasAttributes, IntoAttributeValue, IntoDynNode,
    Template, VComponent, VNode, nodes::IntoVNode,
};
use dioxus_core_template::{
    TEMPLATE_STORAGE_DYNAMIC_CAP, TEMPLATE_STORAGE_OPS_CAP, TEMPLATE_STORAGE_STRING_CAP,
    TemplateRawTree, TemplateStorage,
};

/// A type that contributes static template structure.
pub trait ViewTemplate {
    /// The static tree for this view type.
    const TEMPLATE_TREE: &'static TemplateRawTree;
}

/// Builds the static [`Template`] for a view using template storage capacities resolved at the
/// call site.
pub trait StaticViewTemplateWithCapacity<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
>: ViewTemplate
{
    /// The static template for this view type with the given storage capacities.
    const TEMPLATE: &'static Template;
}

impl<T: ViewTemplate, const OPS_CAP: usize, const STRING_CAP: usize, const DYNAMIC_CAP: usize>
    StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP> for T
{
    const TEMPLATE: &'static Template =
        &TemplateStorage::<OPS_CAP, STRING_CAP, DYNAMIC_CAP>::build_from_tree(T::TEMPLATE_TREE)
            .as_template();
}

impl ViewTemplate for () {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::Empty;
}

/// A typed view that can collect runtime dynamic values.
pub trait View: ViewTemplate + Sized {
    /// Push runtime dynamic values in template order.
    #[inline]
    fn push(self, _dynamic: &mut DynamicValues) {}
}

/// A typed view with a root key.
pub struct KeyedView<V> {
    key: Option<String>,
    view: V,
}

impl<V: ViewTemplate> ViewTemplate for KeyedView<V> {
    const TEMPLATE_TREE: &'static TemplateRawTree = V::TEMPLATE_TREE;
}

impl<V: View> View for KeyedView<V> {
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.set_key(self.key);
        self.view.push(dynamic);
    }
}

/// Extension methods for assigning a root key to typed views.
pub trait ViewKeyExt: View {
    /// Assign a root key to this view.
    fn key(self, key: Option<String>) -> KeyedView<Self>;
}

impl<V: View> ViewKeyExt for V {
    #[inline]
    fn key(self, key: Option<String>) -> KeyedView<Self> {
        KeyedView { key, view: self }
    }
}

/// Extension methods for typed views.
pub trait ViewExt: View {
    /// Convert this view into a [`VNode`].
    fn into_vnode(self) -> VNode;
}

impl<V: View> ViewExt for V {
    #[inline]
    fn into_vnode(self) -> VNode {
        into_vnode_with_template(
            self,
            <V as StaticViewTemplateWithCapacity<
                TEMPLATE_STORAGE_OPS_CAP,
                TEMPLATE_STORAGE_STRING_CAP,
                TEMPLATE_STORAGE_DYNAMIC_CAP,
            >>::TEMPLATE,
        )
    }
}

/// Convert a view into a [`VNode`] using a prepared template.
#[inline]
fn into_vnode_with_template<V: View>(view: V, template: &Template) -> VNode {
    let mut dynamic = DynamicValues::with_capacity(template.dynamic_value_count());
    view.push(&mut dynamic);
    VNode::new(*template, dynamic)
}

/// Convert a view into a [`VNode`] using template capacities resolved at the call site.
#[inline]
pub fn into_vnode_with_capacity<
    const OPS_CAP: usize,
    const STRING_CAP: usize,
    const DYNAMIC_CAP: usize,
    V: View + StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP>,
>(
    view: V,
) -> VNode {
    into_vnode_with_template(
        view,
        <V as StaticViewTemplateWithCapacity<OPS_CAP, STRING_CAP, DYNAMIC_CAP>>::TEMPLATE,
    )
}

impl View for () {}

impl ViewTemplate for VComponent {
    const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicNode;
}

impl View for VComponent {
    #[inline]
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_node(DynamicNode::Component(self));
    }
}

impl IntoVNode for VComponent {
    #[inline]
    fn into_vnode(self) -> VNode {
        ViewExt::into_vnode(self)
    }
}

struct StaticTupleViewChildMarker<T>(PhantomData<fn() -> T>);

macro_rules! impl_tuple_views {
    (($($name:ident $value:ident $marker:ident,)*) ;) => {};
    (($($name:ident $value:ident $marker:ident,)*) ; $next_name:ident $next_value:ident $next_marker:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl $($name $value $marker,)* $next_name $next_value $next_marker,);
        impl_tuple_views!(($($name $value $marker,)* $next_name $next_value $next_marker,) ; $($rest)*);
    };
    (@impl $first_name:ident $first_value:ident $first_marker:ident,) => {
        impl<$first_name: ViewTemplate> ViewTemplate for ($first_name,) {
            const TEMPLATE_TREE: &'static TemplateRawTree = $first_name::TEMPLATE_TREE;
        }

        impl<$first_name: View> View for ($first_name,) {
            #[inline]
            fn push(self, dynamic: &mut DynamicValues) {
                let ($first_value,) = self;
                $first_value.push(dynamic);
            }
        }

    };
    (@impl $first_name:ident $first_value:ident $first_marker:ident, $($name:ident $value:ident $marker:ident,)+) => {
        impl<$first_name: ViewTemplate, $($name: ViewTemplate),*> ViewTemplate for ($first_name, $($name,)*) {
            const TEMPLATE_TREE: &'static TemplateRawTree =
                &TemplateRawTree::Sequence(&[$first_name::TEMPLATE_TREE, $($name::TEMPLATE_TREE,)*]);
        }

        impl<$first_name: View, $($name: View),*> View for ($first_name, $($name,)*) {
            #[inline]
            fn push(self, dynamic: &mut DynamicValues) {
                let ($first_value, $($value,)*) = self;
                $first_value.push(dynamic);
                $($value.push(dynamic);)*
            }
        }

    };
    (@impl_children ($($before_name:ident $before_value:ident,)*) ;) => {};
    (@impl_children ($($before_name:ident $before_value:ident,)*) ; $dynamic_name:ident $dynamic_value:ident $dynamic_marker:ident, $($after_name:ident $after_value:ident $after_marker:ident,)*) => {
        impl_tuple_views!(
            @impl_child_at
            ($($before_name $before_value,)*)
            $dynamic_name $dynamic_value $dynamic_marker
            ($($after_name $after_value $after_marker,)*)
        );
        impl_tuple_views!(
            @impl_children
            ($($before_name $before_value,)* $dynamic_name $dynamic_value,)
            ;
            $($after_name $after_value $after_marker,)*
        );
    };
    (@impl_child_at ($($before_name:ident $before_value:ident,)*) $dynamic_name:ident $dynamic_value:ident $dynamic_marker:ident ($($after_name:ident $after_value:ident $after_marker:ident,)*)) => {
        impl<$($before_name,)* $dynamic_name, $dynamic_marker, $($after_name, $after_marker),*>
            IntoViewChild<($(
                StaticTupleViewChildMarker<$before_name>,
            )* dynamic_node::DynamicViewChildMarker<$dynamic_marker>, $($after_marker,)*)>
            for ($($before_name,)* $dynamic_name, $($after_name,)*)
        where
            $($before_name: View,)*
            $dynamic_name: IntoDynNode<$dynamic_marker>,
            $($after_name: IntoViewChild<$after_marker>),*
        {
            type Output = (
                $($before_name,)*
                dynamic_node::DynamicNodeBuilder<$dynamic_name, $dynamic_marker>,
                $(<$after_name as IntoViewChild<$after_marker>>::Output,)*
            );

            #[inline]
            fn into_child(self) -> Self::Output {
                let ($($before_value,)* $dynamic_value, $($after_value,)*) = self;
                (
                    $($before_value,)*
                    dynamic_node::dynamic_node_builder($dynamic_value),
                    $($after_value.into_child(),)*
                )
            }
        }
    };
}

macro_rules! impl_tuple_view_children {
    (($($name:ident $value:ident $marker:ident,)*) ;) => {};
    (($($name:ident $value:ident $marker:ident,)*) ; $next_name:ident $next_value:ident $next_marker:ident, $($rest:tt)*) => {
        impl_tuple_views!(@impl_children () ; $($name $value $marker,)* $next_name $next_value $next_marker,);
        impl_tuple_view_children!(($($name $value $marker,)* $next_name $next_value $next_marker,) ; $($rest)*);
    };
}

impl_tuple_views! {
    ();
    T00 t00 M00,
    T01 t01 M01,
    T02 t02 M02,
    T03 t03 M03,
    T04 t04 M04,
    T05 t05 M05,
    T06 t06 M06,
    T07 t07 M07,
    T08 t08 M08,
    T09 t09 M09,
    T10 t10 M10,
    T11 t11 M11,
    T12 t12 M12,
    T13 t13 M13,
    T14 t14 M14,
    T15 t15 M15,
    T16 t16 M16,
    T17 t17 M17,
    T18 t18 M18,
    T19 t19 M19,
    T20 t20 M20,
    T21 t21 M21,
    T22 t22 M22,
    T23 t23 M23,
    T24 t24 M24,
    T25 t25 M25,
    T26 t26 M26,
    T27 t27 M27,
    T28 t28 M28,
    T29 t29 M29,
    T30 t30 M30,
    T31 t31 M31,
    T32 t32 M32,
    T33 t33 M33,
    T34 t34 M34,
    T35 t35 M35,
    T36 t36 M36,
    T37 t37 M37,
    T38 t38 M38,
    T39 t39 M39,
    T40 t40 M40,
    T41 t41 M41,
    T42 t42 M42,
    T43 t43 M43,
    T44 t44 M44,
    T45 t45 M45,
    T46 t46 M46,
    T47 t47 M47,
    T48 t48 M48,
    T49 t49 M49,
    T50 t50 M50,
    T51 t51 M51,
    T52 t52 M52,
    T53 t53 M53,
    T54 t54 M54,
    T55 t55 M55,
    T56 t56 M56,
    T57 t57 M57,
    T58 t58 M58,
    T59 t59 M59,
    T60 t60 M60,
    T61 t61 M61,
    T62 t62 M62,
    T63 t63 M63,
    T64 t64 M64,
    T65 t65 M65,
    T66 t66 M66,
    T67 t67 M67,
    T68 t68 M68,
    T69 t69 M69,
    T70 t70 M70,
    T71 t71 M71,
    T72 t72 M72,
    T73 t73 M73,
    T74 t74 M74,
    T75 t75 M75,
    T76 t76 M76,
    T77 t77 M77,
    T78 t78 M78,
    T79 t79 M79,
    T80 t80 M80,
    T81 t81 M81,
    T82 t82 M82,
    T83 t83 M83,
    T84 t84 M84,
    T85 t85 M85,
    T86 t86 M86,
    T87 t87 M87,
    T88 t88 M88,
    T89 t89 M89,
    T90 t90 M90,
    T91 t91 M91,
    T92 t92 M92,
    T93 t93 M93,
    T94 t94 M94,
    T95 t95 M95,
    T96 t96 M96,
    T97 t97 M97,
    T98 t98 M98,
    T99 t99 M99,
    T100 t100 M100,
    T101 t101 M101,
    T102 t102 M102,
    T103 t103 M103,
    T104 t104 M104,
    T105 t105 M105,
    T106 t106 M106,
    T107 t107 M107,
    T108 t108 M108,
    T109 t109 M109,
    T110 t110 M110,
    T111 t111 M111,
    T112 t112 M112,
    T113 t113 M113,
    T114 t114 M114,
    T115 t115 M115,
    T116 t116 M116,
    T117 t117 M117,
    T118 t118 M118,
    T119 t119 M119,
    T120 t120 M120,
    T121 t121 M121,
    T122 t122 M122,
    T123 t123 M123,
    T124 t124 M124,
    T125 t125 M125,
    T126 t126 M126,
    T127 t127 M127,
}

impl_tuple_view_children! {
    ();
    T00 t00 M00,
    T01 t01 M01,
    T02 t02 M02,
    T03 t03 M03,
    T04 t04 M04,
    T05 t05 M05,
    T06 t06 M06,
    T07 t07 M07,
    T08 t08 M08,
    T09 t09 M09,
    T10 t10 M10,
    T11 t11 M11,
    T12 t12 M12,
    T13 t13 M13,
    T14 t14 M14,
    T15 t15 M15,
    T16 t16 M16,
    T17 t17 M17,
    T18 t18 M18,
    T19 t19 M19,
    T20 t20 M20,
    T21 t21 M21,
    T22 t22 M22,
    T23 t23 M23,
    T24 t24 M24,
    T25 t25 M25,
    T26 t26 M26,
    T27 t27 M27,
    T28 t28 M28,
    T29 t29 M29,
    T30 t30 M30,
    T31 t31 M31,
}

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

/// Marker for child values that are already typed views.
#[doc(hidden)]
pub struct ViewChildMarker;

#[doc(hidden)]
pub mod dynamic_node {
    use std::marker::PhantomData;

    use crate::IntoDynNode;
    use dioxus_core_template::TemplateRawTree;

    use super::{DynamicValues, View, ViewTemplate};

    /// Marker for child values that should become dynamic node slots.
    pub struct DynamicViewChildMarker<Marker>(PhantomData<Marker>);

    /// A dynamic node slot.
    pub struct DynamicNodeBuilder<N, Marker = ()> {
        node: N,
        _marker: PhantomData<Marker>,
    }

    /// Create a dynamic node slot from any [`IntoDynNode`] value.
    #[inline]
    pub fn dynamic_node_builder<N, Marker>(node: N) -> DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        DynamicNodeBuilder {
            node,
            _marker: PhantomData,
        }
    }

    impl<N, Marker> ViewTemplate for DynamicNodeBuilder<N, Marker> {
        const TEMPLATE_TREE: &'static TemplateRawTree = &TemplateRawTree::DynamicNode;
    }

    impl<N, Marker> View for DynamicNodeBuilder<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        #[inline]
        fn push(self, dynamic: &mut DynamicValues) {
            dynamic.push_node(self.node.into_dyn_node());
        }
    }
}

/// Convert a value passed to [`ElementBuilder::child`] into a typed child view.
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
    fn push(self, dynamic: &mut DynamicValues) {
        self.children.push(dynamic);
        self.attrs.push(dynamic);
    }
}

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
    #[inline]
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

/// A marker for one static text node.
pub trait StaticText {
    /// Static text value.
    const TEXT: &'static str;
}

/// A static text view.
pub struct StaticTextBuilder<T>(PhantomData<T>);

/// Create a static text view for a text marker.
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

pub use crate::{static_attribute_value, static_text};
