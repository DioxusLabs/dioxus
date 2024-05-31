use std::{any::TypeId, fmt::Arguments};

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
/// #[derive(Props, PartialEq, Clone)]
/// struct MyComponentProps {
///     data: String
/// }
///
/// fn MyComponent(props: MyProps) -> Element {
///     rsx! {
///         div { "Hello {props.data}" }
///     }
/// }
/// ```
///
/// Or even better, derive your entire props struct with the [`#[crate::component]`] macro:
///
/// ```rust
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
    /// The type of the builder for this component.
    /// Used to create "in-progress" versions of the props.
    type Builder;

    /// Create a builder for this component.
    fn builder() -> Self::Builder;

    /// Make the old props equal to the new props. Return if the props were equal and should be memoized.
    fn memoize(&mut self, other: &Self) -> bool;

    /// Create a component from the props.
    fn into_vcomponent<M: 'static>(
        self,
        render_fn: impl ComponentFunction<Self, M>,
        component_name: &'static str,
    ) -> VComponent {
        VComponent::new(render_fn, self, component_name)
    }
}

impl Properties for () {
    type Builder = EmptyBuilder;
    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
    fn memoize(&mut self, _other: &Self) -> bool {
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
    type Builder = P;
    fn builder() -> Self::Builder {
        unreachable!("Root props technically are never built")
    }
    fn memoize(&mut self, _other: &Self) -> bool {
        true
    }
}

// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) {}
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<P, M>(_: impl ComponentFunction<P, M>) -> <P as Properties>::Builder
where
    P: Properties,
{
    P::builder()
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
    /// Get the type id of the component.
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Convert the component to a function that takes props and returns an element.
    fn rebuild(&self, props: Props) -> Element;
}

/// Accept any callbacks that take props
impl<F: Fn(P) -> Element + Clone + 'static, P> ComponentFunction<P> for F {
    fn rebuild(&self, props: P) -> Element {
        self(props)
    }
}

/// Accept any callbacks that take no props
pub struct EmptyMarker;
impl<F: Fn() -> Element + Clone + 'static> ComponentFunction<(), EmptyMarker> for F {
    fn rebuild(&self, _: ()) -> Element {
        self()
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

#[doc(hidden)]
pub struct OptionStringFromMarker;

impl<'a> SuperFrom<&'a str, OptionStringFromMarker> for Option<String> {
    fn super_from(input: &'a str) -> Self {
        Some(String::from(input))
    }
}

#[doc(hidden)]
pub struct OptionArgumentsFromMarker;

impl<'a> SuperFrom<Arguments<'a>, OptionArgumentsFromMarker> for Option<String> {
    fn super_from(input: Arguments<'a>) -> Self {
        Some(input.to_string())
    }
}

#[doc(hidden)]
pub struct OptionHandlerMarker;

impl<G: 'static, F: FnMut(G) + 'static> SuperFrom<F, OptionHandlerMarker>
    for Option<EventHandler<G>>
{
    fn super_from(input: F) -> Self {
        Some(EventHandler::new(input))
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
