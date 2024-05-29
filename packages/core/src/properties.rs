use std::{any::TypeId, fmt::Arguments};

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

#[cfg(debug_assertions)]
thread_local! {
    static CURRENTLY_RUNNING_COMPONENT: std::cell::RefCell<Option<TypeId>> = const { std::cell::RefCell::new(None) };
}

/// Calling a component like a function is a common mistake that can cause issues with out of order hooks and poor performance.
/// In debug mode we try to detect when the user calls a component like a normal function.
///
/// When we call a function, we set a thread local variable to that function pointer. If the component is using the component macro, it will check if the function pointer is set and if it isn't, then the user is calling the component like a function.
#[allow(unused)]
fn call_component_function<O>(type_id: TypeId, call_with: impl FnOnce() -> O) -> O {
    #[cfg(debug_assertions)]
    CURRENTLY_RUNNING_COMPONENT.with(|currently_running_component| {
        currently_running_component.borrow_mut().replace(type_id);
    });

    let result = call_with();

    #[cfg(debug_assertions)]
    CURRENTLY_RUNNING_COMPONENT.with(|currently_running_component| {
        currently_running_component.borrow_mut().take();
    });

    result
}

/// Make sure that this component is currently running as a component, not a function call
#[doc(hidden)]
#[allow(unused)]
pub fn verify_component_called_as_component<C: ComponentFunction<P, M>, P, M>(component: C) {
    #[cfg(debug_assertions)]
    CURRENTLY_RUNNING_COMPONENT.with(|currently_running_component| {
        if let Some(type_id) = currently_running_component.borrow().as_ref() {
            // If we are in a component, and the type id is the same as the component type, then we can just return
            if *type_id == TypeId::of::<C>() {
                return;
            }
        }

        // Otherwise the component was called like a function, so we should log an error
        let type_name = std::any::type_name::<C>();
        tracing::error!(
            "It looks like you called the component {type_name} like a function instead of a component. Components should be called with braces like `{type_name} {{ prop: value }}` instead of as a function"
        );
    });
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

/// Accept any callbacks that take props
impl<F: Fn(P) -> Element + Clone + 'static, P> ComponentFunction<P> for F {
    fn rebuild(&self, props: P) -> Element {
        call_component_function(TypeId::of::<Self>(), || self(props))
    }
}

/// Accept any callbacks that take no props
pub struct EmptyMarker;
impl<F: Fn() -> Element + Clone + 'static> ComponentFunction<(), EmptyMarker> for F {
    fn rebuild(&self, _: ()) -> Element {
        call_component_function(TypeId::of::<Self>(), self)
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
