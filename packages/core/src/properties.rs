use std::{fmt::Arguments, marker::PhantomData};

use crate::innerlude::*;

/// Every "Props" used for a component must implement the `Properties` trait. This trait gives some hints to Dioxus
/// on how to memoize the props and some additional optimizations that can be made. We strongly encourage using the
/// derive macro to implement the `Properties` trait automatically.
///
/// Dioxus requires your props to be 'static, `Clone`, and `PartialEq`. We use the `PartialEq` trait to determine if
/// the props have changed when we diff the component.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// #[derive(Props, PartialEq, Clone)]
/// struct MyComponentProps {
///     data: String
/// }
///
/// fn MyComponent(props: MyComponentProps) -> Element {
///     rsx! {
///         div { "Hello {props.data}" }
///     }
/// }
/// ```
///
/// Or even better, derive your entire props struct with the [`#[crate::component]`] macro:
///
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn MyComponent(data: String) -> Element {
///     rsx! {
///         div { "Hello {data}" }
///     }
/// }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`Props` is not implemented for `{Self}`",
        label = "Props",
        note = "Props is a trait that is automatically implemented for all structs that can be used as props for a component",
        note = "If you manually created a new properties struct, you may have forgotten to add `#[derive(Props, PartialEq, Clone)]` to your struct",
    )
)]
pub trait Properties: Clone + Sized + 'static {
    /// The type of the builder for this component when starting from a component function.
    type ComponentBuilder<RenderFn, Marker>;

    /// Create a builder that remembers the component function it came from.
    fn component_builder<RenderFn, Marker>(
        render_fn: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker>;

    /// Make the old props equal to the new props. Return if the props were equal and should be memoized.
    fn memoize(&mut self, other: &Self) -> bool;

    /// Create a component from the props.
    fn into_vcomponent<M: 'static>(self, render_fn: impl ComponentFunction<Self, M>) -> VComponent {
        let type_name = std::any::type_name_of_val(&render_fn);
        VComponent::new(render_fn, self, type_name)
    }
}

impl Properties for () {
    type ComponentBuilder<RenderFn, Marker> = ComponentBuilder<RenderFn, EmptyBuilder, (), Marker>;

    fn component_builder<RenderFn, Marker>(
        render_fn: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker> {
        ComponentBuilder::new(render_fn, EmptyBuilder {})
    }

    fn memoize(&mut self, _: &Self) -> bool {
        true
    }
}

/// Root properties never need to be memoized, so we can use a dummy implementation.
pub(crate) struct RootProps<P>(pub P);

impl<P> Clone for RootProps<P>
where
    P: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<P> Properties for RootProps<P>
where
    P: Clone + 'static,
{
    type ComponentBuilder<RenderFn, Marker> = ComponentBuilder<RenderFn, P, Self, Marker>;

    fn component_builder<RenderFn, Marker>(
        _: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker> {
        unreachable!("Root props technically are never built")
    }

    fn memoize(&mut self, _: &Self) -> bool {
        true
    }
}

// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) {}
}

/// Any component that implements the `ComponentFn` trait can be used as a component.
///
/// This trait is automatically implemented for functions that are in one of the following forms:
/// - `fn() -> Element`
/// - `fn(props: Properties) -> Element`
///
/// You can derive it automatically for any function with arguments that implement PartialEq with the `#[component]` attribute:
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn MyComponent(a: u32, b: u32) -> Element {
///     rsx! { "a: {a}, b: {b}" }
/// }
/// ```
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`Component<{Props}>` is not implemented for `{Self}`",
        label = "Component",
        note = "Components are functions in the form `fn() -> Element`, `fn(props: Properties) -> Element`, or `#[component] fn(partial_eq1: u32, partial_eq2: u32) -> Element`.",
        note = "You may have forgotten to add `#[component]` to your function to automatically implement the `ComponentFunction` trait."
    )
)]
pub trait ComponentFunction<Props, Marker = ()>: Clone + 'static {
    /// Get the raw address of the component render function.
    fn fn_ptr(&self) -> usize;

    /// Convert the component to a function that takes props and returns an element.
    fn rebuild(&self, props: Props) -> Element;
}

/// Extension methods for function components.
///
/// This lets handwritten builder code start from the component function directly:
///
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn Greeting(#[props(into)] name: String) -> Element {
///     rsx! { "Hello {name}" }
/// }
///
/// let vnode = Greeting.builder().name("Ada").build().into_vnode();
/// ```
pub trait ComponentFunctionExt<Props, Marker>: ComponentFunction<Props, Marker> + Sized
where
    Props: Properties,
{
    /// Create the generated props builder for this component.
    fn builder(self) -> Props::ComponentBuilder<Self, Marker> {
        Props::component_builder(self)
    }
}

impl<F, P, M> ComponentFunctionExt<P, M> for F
where
    F: ComponentFunction<P, M>,
    P: Properties,
{
}

/// A props builder that remembers the component function it came from.
#[must_use]
pub struct ComponentBuilder<RenderFn, Builder, Props, Marker> {
    render_fn: RenderFn,
    builder: Builder,
    _marker: PhantomData<fn() -> (Props, Marker)>,
}

impl<RenderFn, Builder, Props, Marker> ComponentBuilder<RenderFn, Builder, Props, Marker> {
    /// Create a component-aware props builder.
    pub(crate) fn new(render_fn: RenderFn, builder: Builder) -> Self {
        Self {
            render_fn,
            builder,
            _marker: PhantomData,
        }
    }

    /// Convert the inner builder while preserving the component function.
    pub(crate) fn map_builder<NewBuilder>(
        self,
        map: impl FnOnce(Builder) -> NewBuilder,
    ) -> ComponentBuilder<RenderFn, NewBuilder, Props, Marker> {
        ComponentBuilder::new(self.render_fn, map(self.builder))
    }
}

impl<RenderFn, Marker> ComponentBuilder<RenderFn, EmptyBuilder, (), Marker> {
    /// Build an empty-props component.
    pub fn build(self) -> ComponentBuilderOutput<RenderFn, (), Marker> {
        self.builder.build();
        ComponentBuilderOutput::new(self.render_fn, ())
    }
}

impl<RenderFn, Builder, Props, Marker> HasAttributes
    for ComponentBuilder<RenderFn, Builder, Props, Marker>
where
    Builder: HasAttributes,
{
    fn push_attribute<T>(
        self,
        name: &'static str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<T>,
        volatile: bool,
    ) -> Self {
        self.map_builder(|builder| builder.push_attribute(name, ns, attr, volatile))
    }
}

/// A built set of props paired with the component function that renders them.
#[must_use]
pub struct ComponentBuilderOutput<RenderFn, Props, Marker> {
    render_fn: RenderFn,
    props: Props,
    _marker: PhantomData<fn() -> Marker>,
}

impl<RenderFn, Props, Marker> ComponentBuilderOutput<RenderFn, Props, Marker> {
    /// Create built component props from a render function and props value.
    pub fn new(render_fn: RenderFn, props: Props) -> Self {
        Self {
            render_fn,
            props,
            _marker: PhantomData,
        }
    }

    /// Create a [`VComponent`] from these props and the remembered component function.
    pub fn into_vcomponent(self) -> VComponent
    where
        Props: ComponentBuilderRender<RenderFn, Marker>,
    {
        self.props.into_vcomponent(self.render_fn)
    }

    /// Convert this built component into a [`VNode`].
    pub fn into_vnode(self) -> VNode
    where
        Props: ComponentBuilderRender<RenderFn, Marker>,
    {
        crate::view::ViewExt::into_vnode(self.into_vcomponent())
    }
}

impl<RenderFn, Props, Marker> IntoDynNode for ComponentBuilderOutput<RenderFn, Props, Marker>
where
    Props: ComponentBuilderRender<RenderFn, Marker>,
{
    fn into_dyn_node(self) -> DynamicNode {
        DynamicNode::Component(self.into_vcomponent())
    }
}

impl<RenderFn, Props, Marker> IntoVNode for ComponentBuilderOutput<RenderFn, Props, Marker>
where
    Props: ComponentBuilderRender<RenderFn, Marker>,
{
    fn into_vnode(self) -> VNode {
        ComponentBuilderOutput::into_vnode(self)
    }
}

/// Convert built props into a component with a specific render function.
pub trait ComponentBuilderRender<RenderFn, Marker>: Sized {
    /// Create a [`VComponent`] from these props and the render function.
    fn into_vcomponent(self, render_fn: RenderFn) -> VComponent;
}

impl<RenderFn, Marker> ComponentBuilderRender<RenderFn, Marker> for ()
where
    RenderFn: ComponentFunction<(), Marker>,
    Marker: 'static,
{
    fn into_vcomponent(self, render_fn: RenderFn) -> VComponent {
        <Self as Properties>::into_vcomponent(self, render_fn)
    }
}

/// Accept any callbacks that take props
impl<F, P> ComponentFunction<P> for F
where
    F: Fn(P) -> Element + Clone + 'static,
{
    fn rebuild(&self, props: P) -> Element {
        subsecond::HotFn::current(self.clone()).call((props,))
    }

    fn fn_ptr(&self) -> usize {
        subsecond::HotFn::current(self.clone()).ptr_address().0 as usize
    }
}

/// Accept any callbacks that take no props
#[doc(hidden)]
pub struct EmptyMarker;
impl<F> ComponentFunction<(), EmptyMarker> for F
where
    F: Fn() -> Element + Clone + 'static,
{
    fn rebuild(&self, props: ()) -> Element {
        subsecond::HotFn::current(self.clone()).call(props)
    }

    fn fn_ptr(&self) -> usize {
        subsecond::HotFn::current(self.clone()).ptr_address().0 as usize
    }
}

/// A enhanced version of the `Into` trait that allows with more flexibility.
pub trait SuperInto<O, M = ()> {
    /// Convert from a type to another type.
    fn super_into(self) -> O;
}

impl<T, O, M> SuperInto<O, M> for T
where
    O: SuperFrom<T, M>,
{
    fn super_into(self) -> O {
        O::super_from(self)
    }
}

/// A enhanced version of the `From` trait that allows with more flexibility.
pub trait SuperFrom<T, M = ()> {
    /// Convert from a type to another type.
    fn super_from(_: T) -> Self;
}

// first implement for all types that are that implement the From trait
impl<T, O> SuperFrom<T, ()> for O
where
    O: From<T>,
{
    fn super_from(input: T) -> Self {
        Self::from(input)
    }
}

/// Marker used to convert `&str` into `Option<String>` through [`SuperFrom`].
#[doc(hidden)]
pub struct OptionStringFromMarker;

impl<'a> SuperFrom<&'a str, OptionStringFromMarker> for Option<String> {
    fn super_from(input: &'a str) -> Self {
        Some(String::from(input))
    }
}

/// Marker used to convert [`Arguments`] into `Option<String>` through [`SuperFrom`].
#[doc(hidden)]
pub struct OptionArgumentsFromMarker;

impl<'a> SuperFrom<Arguments<'a>, OptionArgumentsFromMarker> for Option<String> {
    fn super_from(input: Arguments<'a>) -> Self {
        Some(input.to_string())
    }
}

/// Marker used to convert a callback into `Option<Callback<_, _>>` through [`SuperFrom`].
#[doc(hidden)]
pub struct OptionCallbackMarker<T>(std::marker::PhantomData<T>);

// Closure can be created from FnMut -> async { anything } or FnMut -> Ret
impl<
    Function: FnMut(Args) -> Spawn + 'static,
    Args: 'static,
    Spawn: SpawnIfAsync<Marker, Ret> + 'static,
    Ret: 'static,
    Marker,
> SuperFrom<Function, OptionCallbackMarker<Marker>> for Option<Callback<Args, Ret>>
{
    fn super_from(input: Function) -> Self {
        Some(Callback::new(input))
    }
}

#[test]
#[allow(unused)]
fn optional_callback_compiles() {
    fn compiles() {
        // Converting from closures (without type hints in the closure works)
        let callback: Callback<i32, i32> = (|num| num * num).super_into();
        let callback: Callback<i32, ()> = (|num| async move { println!("{num}") }).super_into();

        // Converting from closures to optional callbacks works
        let optional: Option<Callback<i32, i32>> = (|num| num * num).super_into();
        let optional: Option<Callback<i32, ()>> =
            (|num| async move { println!("{num}") }).super_into();
    }
}

#[test]
#[allow(unused)]
fn from_props_compiles() {
    // T -> T works
    let option: i32 = 0i32.super_into();
    let option: i32 = 0.super_into(); // Note we don't need type hints on all inputs
    let option: i128 = 0.super_into();
    let option: &'static str = "hello world".super_into();

    // // T -> From<T> works
    let option: i64 = 0i32.super_into();
    let option: String = "hello world".super_into();

    // T -> Option works
    let option: Option<i32> = 0i32.super_into();
    let option: Option<i32> = 0.super_into();
    let option: Option<i128> = 0.super_into();
    fn takes_option_string<M>(_: impl SuperInto<Option<String>, M>) {}
    takes_option_string("hello world");
    takes_option_string("hello world".to_string());
}
