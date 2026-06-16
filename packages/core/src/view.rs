//! Typed, const-driven view builders.
//!
//! This module mirrors the template-v2 builder model: each view type contributes
//! a const raw template tape, and the composed type is promoted to a static
//! [`Template`] through [`Built`].

use std::marker::PhantomData;

use const_vec::ConstVec;

use crate::{
    Attribute, DynamicNode, DynamicValue, HasAttributes, IntoAttributeValue, IntoDynNode, Template,
    TemplateRawAttrNamespace, TemplateRawOp, TemplateStorage, VNode, VText,
};

/// Maximum number of raw template operations a typed view can contribute.
pub const RAW_TAPE_CAP: usize = crate::TEMPLATE_STORAGE_MAX_CAP;

/// A const template-v2-style raw operation tape.
#[derive(Clone, Copy)]
pub struct RawTape {
    ops: ConstVec<TemplateRawOp, RAW_TAPE_CAP>,
}

impl RawTape {
    /// Create an empty raw tape.
    pub const fn new() -> Self {
        Self {
            ops: ConstVec::new_with_max_size(),
        }
    }

    /// Push one raw template operation.
    pub const fn push(&mut self, op: TemplateRawOp) {
        self.ops = self.ops.push(op);
    }

    /// Append another raw tape.
    pub const fn concat(&mut self, other: &RawTape) {
        let mut index = 0;
        while index < other.ops.len() {
            self.ops = self.ops.push(other.ops.at(index));
            index += 1;
        }
    }

    /// Borrow the tape as a static slice during const promotion.
    pub const fn as_slice(&self) -> &[TemplateRawOp] {
        self.ops.as_slice()
    }
}

impl Default for RawTape {
    fn default() -> Self {
        Self::new()
    }
}

/// A type that contributes static template structure.
pub trait Raw {
    /// The raw template-v2-style tape for this view type.
    const RAW: RawTape;
}

impl Raw for () {
    const RAW: RawTape = RawTape::new();
}

macro_rules! impl_raw_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<$($name: Raw),+> Raw for ($($name,)+) {
            const RAW: RawTape = {
                let mut raw = RawTape::new();
                $(raw.concat(&$name::RAW);)+
                raw
            };
        }
    };
}

impl_raw_tuple!(A);
impl_raw_tuple!(A, B);
impl_raw_tuple!(A, B, C);
impl_raw_tuple!(A, B, C, D);
impl_raw_tuple!(A, B, C, D, E);
impl_raw_tuple!(A, B, C, D, E, F);
impl_raw_tuple!(A, B, C, D, E, F, G);
impl_raw_tuple!(A, B, C, D, E, F, G, H);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_raw_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// Type-indexed compile-time static promotion.
pub trait ConstStatic<T: ?Sized + 'static> {
    /// The promoted static value.
    const STATIC: &'static T;
}

impl<V: Raw> ConstStatic<TemplateStorage<RAW_TAPE_CAP>> for V {
    const STATIC: &'static TemplateStorage<RAW_TAPE_CAP> =
        &TemplateStorage::<RAW_TAPE_CAP>::build(V::RAW.as_slice());
}

impl<V: Raw> ConstStatic<Template> for V {
    const STATIC: &'static Template =
        &<Self as ConstStatic<TemplateStorage<RAW_TAPE_CAP>>>::STATIC.as_template();
}

/// A type with a promoted static template.
pub trait Built: Raw + ConstStatic<Template> {
    /// The promoted static template for this view type.
    const TEMPLATE: &'static Template = <Self as ConstStatic<Template>>::STATIC;
}

impl<V: Raw> Built for V {}

/// Runtime dynamic values collected while consuming a typed view.
#[derive(Debug, Default)]
pub struct DynamicValues {
    values: Vec<DynamicValue>,
}

impl DynamicValues {
    /// Create an empty dynamic-value buffer.
    pub const fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Push a dynamic node slot.
    pub fn push_node(&mut self, value: DynamicNode) {
        self.values.push(DynamicValue::Node(value));
    }

    /// Push a dynamic attribute slot.
    pub fn push_attrs(&mut self, value: Box<[Attribute]>) {
        self.values.push(DynamicValue::Attrs(value));
    }

    /// Convert this buffer into the boxed slice expected by [`VNode`].
    pub fn into_boxed_slice(self) -> Box<[DynamicValue]> {
        self.values.into_boxed_slice()
    }
}

/// A typed view that can collect runtime dynamic values.
pub trait View: Raw + Built + Sized {
    /// Push runtime dynamic values in template order.
    fn push(self, _dynamic: &mut DynamicValues) {}

    /// Convert this view into a [`VNode`].
    fn into_vnode(self) -> VNode {
        let mut dynamic = DynamicValues::new();
        self.push(&mut dynamic);
        VNode::new(None, *Self::TEMPLATE, dynamic.into_boxed_slice())
    }
}

impl View for () {}

macro_rules! impl_view_tuple {
    ($(($name:ident, $value:ident)),+ $(,)?) => {
        impl<$($name: View),+> View for ($($name,)+) {
            fn push(self, dynamic: &mut DynamicValues) {
                let ($($value,)+) = self;
                $($value.push(dynamic);)+
            }
        }
    };
}

impl_view_tuple!((A, a));
impl_view_tuple!((A, a), (B, b));
impl_view_tuple!((A, a), (B, b), (C, c));
impl_view_tuple!((A, a), (B, b), (C, c), (D, d));
impl_view_tuple!((A, a), (B, b), (C, c), (D, d), (E, e));
impl_view_tuple!((A, a), (B, b), (C, c), (D, d), (E, e), (F, f));
impl_view_tuple!((A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g));
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m),
    (N, n)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m),
    (N, n),
    (O, o)
);
impl_view_tuple!(
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m),
    (N, n),
    (O, o),
    (P, p)
);

/// A static element tag marker.
pub trait TagName {
    /// The renderer tag name.
    const NAME: &'static str;

    /// The optional renderer namespace.
    const NAMESPACE: Option<&'static str> = None;
}

/// A typed element view.
pub struct El<Tag, Attrs, Children> {
    attrs: Attrs,
    children: Children,
    _tag: PhantomData<Tag>,
}

/// Create an empty typed element for a tag marker.
pub const fn el<Tag>() -> El<Tag, (), ()> {
    El {
        attrs: (),
        children: (),
        _tag: PhantomData,
    }
}

impl<Tag, Attrs, Children> El<Tag, Attrs, Children> {
    /// Append one attribute view.
    pub fn attr<Attr>(self, attr: Attr) -> El<Tag, (Attrs, Attr), Children> {
        El {
            attrs: (self.attrs, attr),
            children: self.children,
            _tag: PhantomData,
        }
    }

    /// Append one child.
    pub fn child<Child, Marker>(
        self,
        child: Child,
    ) -> El<Tag, Attrs, (Children, <Child as IntoChild<Marker>>::Output)>
    where
        Child: IntoChild<Marker>,
    {
        El {
            attrs: self.attrs,
            children: (self.children, child.into_child()),
            _tag: PhantomData,
        }
    }
}

/// Marker for child values that are already typed views.
pub struct ViewChild;

pub(crate) mod dynamic_node {
    use std::marker::PhantomData;

    use crate::{IntoDynNode, TemplateRawOp};

    use super::{DynamicValues, Raw, RawTape, View};

    /// Marker for child values that should become dynamic node slots.
    pub struct DynamicChild<Marker>(PhantomData<Marker>);

    /// A dynamic node slot.
    pub struct DynNode<N, Marker = ()> {
        node: N,
        _marker: PhantomData<Marker>,
    }

    /// Create a dynamic node slot from any [`IntoDynNode`] value.
    pub fn node_dyn<N, Marker>(node: N) -> DynNode<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        DynNode {
            node,
            _marker: PhantomData,
        }
    }

    impl<N, Marker> Raw for DynNode<N, Marker> {
        const RAW: RawTape = {
            let mut raw = RawTape::new();
            raw.push(TemplateRawOp::DynamicNode);
            raw
        };
    }

    impl<N, Marker> View for DynNode<N, Marker>
    where
        N: IntoDynNode<Marker>,
    {
        fn push(self, dynamic: &mut DynamicValues) {
            dynamic.push_node(self.node.into_dyn_node());
        }
    }
}

/// Convert a value passed to [`El::child`] into a typed child view.
pub trait IntoChild<Marker = ViewChild> {
    /// The typed view contributed by this child.
    type Output: View;

    /// Convert into the child view.
    fn into_child(self) -> Self::Output;
}

impl<V: View> IntoChild<ViewChild> for V {
    type Output = V;

    fn into_child(self) -> Self::Output {
        self
    }
}

impl<N, Marker> IntoChild<dynamic_node::DynamicChild<Marker>> for N
where
    N: IntoDynNode<Marker>,
{
    type Output = dynamic_node::DynNode<N, Marker>;

    fn into_child(self) -> Self::Output {
        dynamic_node::node_dyn(self)
    }
}

impl<Tag: TagName, Attrs: Raw, Children: Raw> Raw for El<Tag, Attrs, Children> {
    const RAW: RawTape = {
        let mut raw = RawTape::new();
        raw.push(TemplateRawOp::OpenElement {
            tag: Tag::NAME,
            namespace: Tag::NAMESPACE,
        });
        raw.concat(&Attrs::RAW);
        raw.concat(&Children::RAW);
        raw.push(TemplateRawOp::CloseElement);
        raw
    };
}

impl<Tag: TagName, Attrs: View, Children: View> View for El<Tag, Attrs, Children> {
    fn push(self, dynamic: &mut DynamicValues) {
        self.attrs.push(dynamic);
        self.children.push(dynamic);
    }
}

/// A marker for one static attribute.
pub trait StaticAttribute {
    /// Attribute name.
    const NAME: &'static str;

    /// Attribute value.
    const VALUE: &'static str;

    /// Attribute namespace.
    const NAMESPACE: TemplateRawAttrNamespace = None;
}

/// Static metadata for a generated attribute builder method.
pub trait AttributeDescriptor {
    /// Attribute name.
    const NAME: &'static str;

    /// Attribute namespace.
    const NAMESPACE: TemplateRawAttrNamespace = None;

    /// Whether this dynamic attribute should always be written.
    const VOLATILE: bool = false;
}

/// A static attribute view.
pub struct Attr<A>(PhantomData<A>);

/// Create a static attribute view for an attribute marker.
pub const fn attr<A: StaticAttribute>() -> Attr<A> {
    Attr(PhantomData)
}

impl<A: StaticAttribute> Raw for Attr<A> {
    const RAW: RawTape = {
        let mut raw = RawTape::new();
        raw.push(TemplateRawOp::StaticAttr {
            name: A::NAME,
            value: A::VALUE,
            namespace: A::NAMESPACE,
        });
        raw
    };
}

impl<A: StaticAttribute> View for Attr<A> {}

/// A marker for one static attribute value.
pub trait StaticAttributeValue {
    /// Attribute value.
    const VALUE: &'static str;
}

/// A static attribute value that can be passed to generated attribute builder methods.
pub struct StaticValue<V>(PhantomData<V>);

/// Create a static attribute value from a marker type.
pub const fn static_value<V: StaticAttributeValue>() -> StaticValue<V> {
    StaticValue(PhantomData)
}

/// A static attribute assembled from a generated descriptor and a static value.
pub struct StaticAttr<Descriptor, Value>(PhantomData<(Descriptor, Value)>);

impl<Descriptor, Value> StaticAttribute for StaticAttr<Descriptor, Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    const NAME: &'static str = Descriptor::NAME;
    const VALUE: &'static str = Value::VALUE;
    const NAMESPACE: TemplateRawAttrNamespace = Descriptor::NAMESPACE;
}

#[doc(hidden)]
pub struct StaticAttributeBuilderMarker;

/// A value that can be appended by a generated attribute builder method.
pub trait IntoAttributeBuilderValue<Target, Descriptor, Marker>
where
    Target: AttributeTarget,
    Descriptor: AttributeDescriptor,
{
    /// The target returned after appending this attribute value.
    type Output;

    /// Append this value to the target.
    fn append_to(self, target: Target) -> Self::Output;
}

/// A dynamic attribute slot.
pub struct DynAttrs {
    attrs: Box<[Attribute]>,
}

/// Create a dynamic attribute slot from an already boxed attribute list.
pub(crate) fn attrs_dyn(attrs: Box<[Attribute]>) -> DynAttrs {
    DynAttrs { attrs }
}

/// Create a dynamic attribute slot with a single attribute.
pub fn attr_dyn<T>(
    name: &'static str,
    value: impl IntoAttributeValue<T>,
    namespace: Option<&'static str>,
    volatile: bool,
) -> DynAttrs {
    DynAttrs {
        attrs: Box::new([Attribute::new(name, value, namespace, volatile)]),
    }
}

impl Raw for DynAttrs {
    const RAW: RawTape = {
        let mut raw = RawTape::new();
        raw.push(TemplateRawOp::DynamicAttr);
        raw
    };
}

impl View for DynAttrs {
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_attrs(self.attrs);
    }
}

/// A builder target that can accept one dynamic attribute.
pub trait AttributeTarget: Sized {
    /// The target returned after adding the attribute.
    type Output;

    /// Append one fully constructed dynamic attribute.
    fn append_attribute(self, attr: Attribute) -> Self::Output;

    /// Build and append one dynamic attribute from its parts.
    fn push_dynamic_attribute<T>(
        self,
        name: &'static str,
        namespace: Option<&'static str>,
        value: impl IntoAttributeValue<T>,
        volatile: bool,
    ) -> Self::Output {
        self.append_attribute(Attribute::new(name, value, namespace, volatile))
    }
}

impl<Target> AttributeTarget for Target
where
    Target: HasAttributes,
{
    type Output = Self;

    fn append_attribute(self, attr: Attribute) -> Self::Output {
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }
}

impl<Tag, Attrs, Children> AttributeTarget for El<Tag, Attrs, Children> {
    type Output = El<Tag, (Attrs, DynAttrs), Children>;

    fn append_attribute(self, attr: Attribute) -> Self::Output {
        self.attr(attrs_dyn(Box::new([attr])))
    }
}

impl<Target, Descriptor, Marker, Value> IntoAttributeBuilderValue<Target, Descriptor, Marker>
    for Value
where
    Target: AttributeTarget,
    Descriptor: AttributeDescriptor,
    Value: IntoAttributeValue<Marker>,
{
    type Output = <Target as AttributeTarget>::Output;

    fn append_to(self, target: Target) -> Self::Output {
        AttributeTarget::push_dynamic_attribute(
            target,
            Descriptor::NAME,
            Descriptor::NAMESPACE,
            self,
            Descriptor::VOLATILE,
        )
    }
}

impl<Tag, Attrs, Children, Descriptor, Value>
    IntoAttributeBuilderValue<El<Tag, Attrs, Children>, Descriptor, StaticAttributeBuilderMarker>
    for StaticValue<Value>
where
    Descriptor: AttributeDescriptor,
    Value: StaticAttributeValue,
{
    type Output = El<Tag, (Attrs, Attr<StaticAttr<Descriptor, Value>>), Children>;

    fn append_to(self, target: El<Tag, Attrs, Children>) -> Self::Output {
        target.attr(attr::<StaticAttr<Descriptor, Value>>())
    }
}

/// A marker for one static text node.
pub trait StaticText {
    /// Static text value.
    const TEXT: &'static str;
}

/// A static text view.
pub struct Text<T>(PhantomData<T>);

/// Create a static text view for a text marker.
pub const fn text<T: StaticText>() -> Text<T> {
    Text(PhantomData)
}

impl<T: StaticText> Raw for Text<T> {
    const RAW: RawTape = {
        let mut raw = RawTape::new();
        raw.push(TemplateRawOp::StaticText { value: T::TEXT });
        raw
    };
}

impl<T: StaticText> View for Text<T> {}

/// A dynamic text node.
pub struct DynText {
    value: String,
}

/// Create a dynamic text node.
pub fn text_dyn(value: impl ToString) -> DynText {
    DynText {
        value: value.to_string(),
    }
}

impl Raw for DynText {
    const RAW: RawTape = {
        let mut raw = RawTape::new();
        raw.push(TemplateRawOp::DynamicNode);
        raw
    };
}

impl View for DynText {
    fn push(self, dynamic: &mut DynamicValues) {
        dynamic.push_node(DynamicNode::Text(VText::new(self.value)));
    }
}

#[doc(hidden)]
pub(crate) fn internal_node_dyn<N, Marker>(node: N) -> dynamic_node::DynNode<N, Marker>
where
    N: IntoDynNode<Marker>,
{
    dynamic_node::node_dyn(node)
}

#[doc(hidden)]
pub(crate) fn internal_attrs_dyn(attrs: Box<[Attribute]>) -> DynAttrs {
    attrs_dyn(attrs)
}

/// A typed view with a root key.
pub struct Keyed<V> {
    view: V,
    key: Option<String>,
}

/// Convert a value into an optional root key.
pub trait IntoKey {
    /// Convert this value into an optional key.
    fn into_key(self) -> Option<String>;
}

impl IntoKey for String {
    fn into_key(self) -> Option<String> {
        Some(self)
    }
}

impl IntoKey for &str {
    fn into_key(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoKey for Option<String> {
    fn into_key(self) -> Option<String> {
        self
    }
}

/// Attach a key to a typed view.
pub fn keyed<V>(view: V, key: impl IntoKey) -> Keyed<V> {
    Keyed {
        view,
        key: key.into_key(),
    }
}

impl<V: Raw> Raw for Keyed<V> {
    const RAW: RawTape = V::RAW;
}

impl<V: View> View for Keyed<V> {
    fn push(self, dynamic: &mut DynamicValues) {
        self.view.push(dynamic);
    }

    fn into_vnode(self) -> VNode {
        let key = self.key;
        let mut dynamic = DynamicValues::new();
        self.view.push(&mut dynamic);
        VNode::new(key, *Self::TEMPLATE, dynamic.into_boxed_slice())
    }
}

/// Declare a static text marker type.
#[macro_export]
macro_rules! static_text {
    ($value:literal) => {{
        #[doc(hidden)]
        mod __dioxus_static_text {
            pub(super) fn view() -> impl $crate::view::View {
                struct Text;
                impl $crate::view::StaticText for Text {
                    const TEXT: &'static str = $value;
                }

                $crate::view::text::<Text>()
            }
        }

        __dioxus_static_text::view()
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

/// Declare a static attribute marker type.
#[macro_export]
macro_rules! static_attribute {
    ($attr:literal, $value:literal) => {{
        #[doc(hidden)]
        mod __dioxus_static_attribute {
            pub(super) fn view() -> impl $crate::view::View {
                struct Attribute;
                impl $crate::view::StaticAttribute for Attribute {
                    const NAME: &'static str = $attr;
                    const VALUE: &'static str = $value;
                }

                $crate::view::attr::<Attribute>()
            }
        }

        __dioxus_static_attribute::view()
    }};
    ($attr:literal, $value:literal, $namespace:expr) => {{
        #[doc(hidden)]
        mod __dioxus_static_attribute {
            pub(super) fn view() -> impl $crate::view::View {
                struct Attribute;
                impl $crate::view::StaticAttribute for Attribute {
                    const NAME: &'static str = $attr;
                    const VALUE: &'static str = $value;
                    const NAMESPACE: $crate::TemplateRawAttrNamespace = $namespace;
                }

                $crate::view::attr::<Attribute>()
            }
        }

        __dioxus_static_attribute::view()
    }};
    ($name:ident, $attr:literal, $value:literal) => {
        $crate::static_attribute!(pub struct $name, $attr, $value);
    };
    ($name:ident, $attr:literal, $value:literal, $namespace:expr) => {
        $crate::static_attribute!(pub struct $name, $attr, $value, $namespace);
    };
    ($vis:vis struct $name:ident, $attr:expr, $value:expr) => {
        $vis struct $name;
        impl $crate::view::StaticAttribute for $name {
            const NAME: &'static str = $attr;
            const VALUE: &'static str = $value;
        }
    };
    ($vis:vis struct $name:ident, $attr:expr, $value:expr, $namespace:expr) => {
        $vis struct $name;
        impl $crate::view::StaticAttribute for $name {
            const NAME: &'static str = $attr;
            const VALUE: &'static str = $value;
            const NAMESPACE: $crate::TemplateRawAttrNamespace = $namespace;
        }
    };
}

/// Declare a static attribute value for generated attribute builder methods.
#[macro_export]
macro_rules! static_value {
    ($value:literal) => {{
        #[doc(hidden)]
        mod __dioxus_static_value {
            pub(super) struct Value;

            impl $crate::view::StaticAttributeValue for Value {
                const VALUE: &'static str = $value;
            }

            pub(super) fn value() -> $crate::view::StaticValue<Value> {
                $crate::view::static_value::<Value>()
            }
        }

        __dioxus_static_value::value()
    }};
    ($name:ident, $value:literal) => {
        $crate::static_value!(pub struct $name, $value);
    };
    ($vis:vis struct $name:ident, $value:expr) => {
        $vis struct $name;
        impl $crate::view::StaticAttributeValue for $name {
            const VALUE: &'static str = $value;
        }
    };
}

/// Compatibility alias for [`static_attribute!`].
#[macro_export]
macro_rules! static_attr {
    ($($tokens:tt)*) => {
        $crate::static_attribute!($($tokens)*);
    };
}
