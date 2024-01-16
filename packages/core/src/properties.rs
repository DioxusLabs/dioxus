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

    /// Memoization can only happen if the props are valid for the 'static lifetime
    ///
    /// # Safety
    /// The user must know if their props are static, but if they make a mistake, UB happens
    /// Therefore it's unsafe to memoize.
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
pub fn fc_to_builder<F: ComponentFunction<P>, P>(
    _: F,
) -> <<F as ComponentFunction<P>>::Props as Properties>::Builder
where
    F::Props: Properties,
{
    F::Props::builder()
}

/// Every component used in rsx must implement the `ComponentFunction` trait. This trait tells dioxus how your component should be rendered.
///
/// Dioxus automatically implements this trait for any function that either takes no arguments or a single props argument and returns an Element.
///
/// ## Example
///
/// For components that take no props:
///
/// ```rust
/// fn app() -> Element {
///     rsx! {
///         div {}
///     }
/// }
/// ```
///
/// For props that take a props struct:
///
/// ```rust
/// #[derive(Props, PartialEq, Clone)]
/// struct MyProps {
///    data: String
/// }
///
/// fn app(props: MyProps) -> Element {
///     rsx! {
///         div {
///             "{props.data}"
///         }
///     }
/// }
/// ```
///
/// Or you can use the #[component] macro to automatically implement create the props struct:
///
/// ```rust
/// #[component]
/// fn app(data: String) -> Element {
///     rsx! {
///         div {
///             "{data}"
///         }
///     }
/// }
/// ```
///
/// > Note: If you get an error about the `ComponentFunction` trait not being implemented: make sure your props implements the `Properties` trait or if you would like to declare your props inline, make sure you use the #[component] macro on your function.
pub trait ComponentFunction<P>: Clone + 'static {
    /// The props type for this component.
    type Props: 'static;

    /// Run the component function with the given props.
    fn call(&self, props: Self::Props) -> Element;
}

impl<T: 'static, F: Fn(T) -> Element + Clone + 'static> ComponentFunction<(T,)> for F {
    type Props = T;

    fn call(&self, props: T) -> Element {
        self(props)
    }
}

#[doc(hidden)]
pub struct ZeroElementMarker;
impl<F: Fn() -> Element + Clone + 'static> ComponentFunction<ZeroElementMarker> for F {
    type Props = ();

    fn call(&self, _: ()) -> Element {
        self()
    }
}

#[test]
fn test_empty_builder() {
    fn app() -> Element {
        unimplemented!()
    }
    fn app2(_: ()) -> Element {
        unimplemented!()
    }
    let builder = fc_to_builder(app);
    builder.build();
    let builder = fc_to_builder(app2);
    builder.build();
}
