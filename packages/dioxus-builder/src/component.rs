//! Component helpers for typed props builders.

use dioxus_core::{Element, Properties};

/// Alias trait for Dioxus props with builder support.
///
/// This mirrors the `Props` naming from derives, while delegating to
/// the core [`Properties`] trait for memoization and type safety.
pub trait Props: Properties {
    /// The type of the builder for this component's props.
    type Builder;

    /// Create a builder for this component's props.
    fn builder() -> <Self as Props>::Builder;
}

impl<T> Props for T
where
    T: Properties,
{
    type Builder = <T as Properties>::Builder;

    fn builder() -> <Self as Props>::Builder {
        <T as Properties>::builder()
    }
}

/// Enables `Component.new().field(x).build()` syntax for props builders.
///
/// This works with any props type that implements [`Properties`], including
/// bon-generated builders with a manual `Properties` impl.
#[allow(clippy::wrong_self_convention, clippy::new_ret_no_self)]
pub trait FunctionComponent<P: Props> {
    /// Create a new builder for this component's props.
    fn new(&self) -> <P as Props>::Builder;
}

impl<P, F> FunctionComponent<P> for F
where
    F: Fn(P) -> Element + Clone + 'static,
    P: Props,
{
    fn new(&self) -> <P as Props>::Builder {
        <P as Props>::builder()
    }
}
