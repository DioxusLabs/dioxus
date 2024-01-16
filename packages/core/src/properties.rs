use std::rc::Rc;

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

// We allow components to use the () generic parameter if they have no props. This impl enables the "build" method
// that the macros use to anonymously complete prop construction.
pub struct EmptyBuilder;
impl EmptyBuilder {
    pub fn build(self) {}
}

/// This utility function launches the builder method so rsx! and html! macros can use the typed-builder pattern
/// to initialize a component's props.
pub fn fc_to_builder<P, M>(_: impl ComponentFn<P, M>) -> <P as Properties>::Builder
where
    P: Properties,
{
    P::builder()
}

/// Any component that implements the `ComponentFn` trait can be used as a component.
pub trait ComponentFn<Props, Marker> {
    /// Convert the component to a function that takes props and returns an element.
    fn as_component(self: Rc<Self>) -> Component<Props>;
}

/// Accept pre-formed component render functions as components
impl<P> ComponentFn<P, ()> for Component<P> {
    fn as_component(self: Rc<Self>) -> Component<P> {
        self.as_ref().clone()
    }
}

/// Accept any callbacks that take props
impl<F: Fn(P) -> Element + 'static, P> ComponentFn<P, ()> for F {
    fn as_component(self: Rc<Self>) -> Component<P> {
        self
    }
}

/// Accept any callbacks that take no props
pub struct EmptyMarker;
impl<F: Fn() -> Element + 'static> ComponentFn<(), EmptyMarker> for F {
    fn as_component(self: Rc<Self>) -> Rc<dyn Fn(()) -> Element> {
        Rc::new(move |_| self())
    }
}

#[test]
fn it_works_maybe() {
    fn test(_: ()) -> Element {
        todo!()
    }
    fn test2() -> Element {
        todo!()
    }

    let callable: Rc<dyn ComponentFn<(), ()>> = Rc::new(test) as Rc<dyn ComponentFn<_, _>>;
    let callable2: Rc<dyn ComponentFn<(), EmptyMarker>> =
        Rc::new(test2) as Rc<dyn ComponentFn<_, _>>;
}
