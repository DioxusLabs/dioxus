use std::any::TypeId;

use crate::innerlude::*;

/// Every "Props" used for a component must implement the `Properties` trait. This trait gives some hints to Dioxus
/// on how to memoize the props and some additional optimizations that can be made. We strongly encourage using the
/// derive macro to implement the `Properties` trait automatically as guarantee that your memoization strategy is safe.
///
/// If your props are 'static, then Dioxus will require that they also be PartialEq for the derived memoize strategy.
///
/// By default, the memoization strategy is very conservative, but can be tuned to be more aggressive manually. However,
/// this is only safe if the props are 'static - otherwise you might borrow references after-free.
///
/// We strongly suggest that any changes to memoization be done at the "PartialEq" level for 'static props. Additionally,
/// we advise the use of smart pointers in cases where memoization is important.
///
/// ## Example
///
/// For props that are 'static:
/// ```rust, ignore
/// #[derive(Props, PartialEq, Clone)]
/// struct MyProps {
///     data: String
/// }
/// ```
pub trait Properties: Clone + Sized + 'static {
    /// The type of the builder for this component.
    /// Used to create "in-progress" versions of the props.
    type Builder;

    /// Create a builder for this component.
    fn builder() -> Self::Builder;

    /// Compare two props to see if they are memoizable.
    fn memoize(&self, other: &Self) -> bool;
}

impl Properties for () {
    type Builder = EmptyBuilder;
    fn builder() -> Self::Builder {
        EmptyBuilder {}
    }
    fn memoize(&self, _other: &Self) -> bool {
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
        todo!()
    }
    fn memoize(&self, _other: &Self) -> bool {
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
pub trait ComponentFunction<Props, Marker = ()>: Clone + 'static {
    /// Get the type id of the component.
    fn id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    /// Convert the component to a function that takes props and returns an element.
    fn rebuild(&self, props: Props) -> Element;
}

/// Accept pre-formed component render functions as components
impl<P: 'static> ComponentFunction<P> for Component<P> {
    fn rebuild(&self, props: P) -> Element {
        (self)(props)
    }
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
