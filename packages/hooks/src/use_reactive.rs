use dioxus_signals::{Readable, Writable};

use crate::use_signal;

/// A dependency is a trait that can be used to determine if a effect or selector should be re-run.
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`Dependency` is not implemented for `{Self}`",
        label = "Dependency",
        note = "Dependency is automatically implemented for all tuples with less than 8 references to element that implement `PartialEq` and `Clone`. For example, `(&A, &B, &C)` implements `Dependency` automatically as long as `A`, `B`, and `C` implement `PartialEq` and `Clone`.",
    )
)]
pub trait Dependency: Sized + Clone {
    /// The output of the dependency
    type Out: Clone + PartialEq + 'static;
    /// Returns the output of the dependency.
    fn out(&self) -> Self::Out;
    /// Returns true if the dependency has changed.
    fn changed(&self, other: &Self::Out) -> bool {
        self.out() != *other
    }
}

impl Dependency for () {
    type Out = ();
    fn out(&self) -> Self::Out {}
}

/// A dependency is a trait that can be used to determine if a effect or selector should be re-run.
#[rustversion::attr(
    since(1.78.0),
    diagnostic::on_unimplemented(
        message = "`DependencyElement` is not implemented for `{Self}`",
        label = "dependency element",
        note = "DependencyElement is automatically implemented for types that implement `PartialEq` and `Clone`",
    )
)]
pub trait DependencyElement: 'static + PartialEq + Clone {}
impl<T> DependencyElement for T where T: 'static + PartialEq + Clone {}

impl<A: DependencyElement> Dependency for &A {
    type Out = A;
    fn out(&self) -> Self::Out {
        (*self).clone()
    }
}

macro_rules! impl_dep {
    (
        $($el:ident=$name:ident $other:ident,)*
    ) => {
        impl< $($el),* > Dependency for ($(&$el,)*)
        where
            $(
                $el: DependencyElement
            ),*
        {
            type Out = ($($el,)*);

            fn out(&self) -> Self::Out {
                let ($($name,)*) = self;
                ($((*$name).clone(),)*)
            }

            fn changed(&self, other: &Self::Out) -> bool {
                let ($($name,)*) = self;
                let ($($other,)*) = other;
                $(
                    if *$name != $other {
                        return true;
                    }
                )*
                false
            }
        }
    };
}

impl_dep!(A = a1 a2,);
impl_dep!(A = a1 a2, B = b1 b2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2, G = g1 g2,);
impl_dep!(A = a1 a2, B = b1 b2, C = c1 c2, D = d1 d2, E = e1 e2, F = f1 f2, G = g1 g2, H = h1 h2,);

/// Takes some non-reactive data, and a closure and returns a closure that will subscribe to that non-reactive data as if it were reactive.
///
/// # Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// let data = 5;
///
/// use_effect(use_reactive((&data,), |(data,)| {
///     println!("Data changed: {}", data);
/// }));
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
pub fn use_reactive<O, D: Dependency>(
    non_reactive_data: D,
    mut closure: impl FnMut(D::Out) -> O + 'static,
) -> impl FnMut() -> O + 'static {
    let mut first_run = false;
    let mut last_state = use_signal(|| {
        first_run = true;
        non_reactive_data.out()
    });
    if !first_run && non_reactive_data.changed(&*last_state.peek()) {
        last_state.set(non_reactive_data.out());
    }
    move || closure(last_state())
}

/// A helper macro for `use_reactive` that merges uses the closure syntax to elaborate the dependency array
///
/// Takes some non-reactive data, and a closure and returns a closure that will subscribe to that non-reactive data as if it were reactive.
///
/// # Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// let data = 5;
///
/// use_effect(use_reactive!(|data| {
///     println!("Data changed: {}", data);
/// }));
/// ```
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[macro_export]
macro_rules! use_reactive {
    (|| $($rest:tt)*) => { use_reactive( (), move |_| $($rest)* ) };
    (| $($args:tt),* | $($rest:tt)*) => {
        use_reactive(
            ($(&$args),*),
            move |($($args),*)| $($rest)*
        )
    };
}
