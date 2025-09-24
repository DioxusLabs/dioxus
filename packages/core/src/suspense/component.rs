use crate::{innerlude::*, scope_context::SuspenseLocation};

/// Properties for the [`SuspenseBoundary()`] component.
#[allow(non_camel_case_types)]
pub struct SuspenseBoundaryProps {
    pub(crate) fallback: Callback<SuspenseContext, Element>,
    /// The children of the suspense boundary
    pub(crate) children: Element,
}

impl Clone for SuspenseBoundaryProps {
    fn clone(&self) -> Self {
        Self {
            fallback: self.fallback,
            children: self.children.clone(),
        }
    }
}

impl SuspenseBoundaryProps {
    /**
    Create a builder for building `SuspenseBoundaryProps`.
    On the builder, call `.fallback(...)`, `.children(...)`(optional) to set the values of the fields.
    Finally, call `.build()` to create the instance of `SuspenseBoundaryProps`.
                        */
    #[allow(dead_code, clippy::type_complexity)]
    fn builder() -> SuspenseBoundaryPropsBuilder<((), ())> {
        SuspenseBoundaryPropsBuilder {
            owner: Owner::default(),
            fields: ((), ()),
            _phantom: ::core::default::Default::default(),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct SuspenseBoundaryPropsBuilder<TypedBuilderFields> {
    owner: Owner,
    fields: TypedBuilderFields,
    _phantom: (),
}
impl Properties for SuspenseBoundaryProps
where
    Self: Clone,
{
    type Builder = SuspenseBoundaryPropsBuilder<((), ())>;
    fn builder() -> Self::Builder {
        SuspenseBoundaryProps::builder()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        self.fallback.__point_to(&new.fallback);
        if !equal {
            let new_clone = new.clone();
            self.children = new_clone.children;
        }
        equal
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait SuspenseBoundaryPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> SuspenseBoundaryPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> SuspenseBoundaryPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> SuspenseBoundaryPropsBuilder<((), __children)> {
    #[allow(clippy::type_complexity)]
    pub fn fallback<__Marker>(
        self,
        fallback: impl SuperInto<Callback<SuspenseContext, Element>, __Marker>,
    ) -> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
        let fallback = (with_owner(self.owner.clone(), move || {
            SuperInto::super_into(fallback)
        }),);
        let (_, children) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_fallback {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
    #[deprecated(note = "Repeated field fallback")]
    #[allow(clippy::type_complexity)]
    pub fn fallback(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Repeated_field_fallback,
    ) -> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__fallback> SuspenseBoundaryPropsBuilder<(__fallback, ())> {
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        children: Element,
    ) -> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
        let children = (children,);
        let (fallback, _) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_children {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__fallback> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
    #[deprecated(note = "Repeated field children")]
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Repeated_field_children,
    ) -> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
        self
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Missing_required_field_fallback {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
impl<__children> SuspenseBoundaryPropsBuilder<((), __children)> {
    #[deprecated(note = "Missing required field fallback")]
    pub fn build(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Missing_required_field_fallback,
    ) -> SuspenseBoundaryProps {
        panic!()
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
pub struct SuspenseBoundaryPropsWithOwner {
    inner: SuspenseBoundaryProps,
    owner: Owner,
}
#[automatically_derived]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl ::core::clone::Clone for SuspenseBoundaryPropsWithOwner {
    #[inline]
    fn clone(&self) -> SuspenseBoundaryPropsWithOwner {
        SuspenseBoundaryPropsWithOwner {
            inner: ::core::clone::Clone::clone(&self.inner),
            owner: ::core::clone::Clone::clone(&self.owner),
        }
    }
}
impl PartialEq for SuspenseBoundaryPropsWithOwner {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}
impl SuspenseBoundaryPropsWithOwner {
    /// Create a component from the props.
    pub fn into_vcomponent<M: 'static>(
        self,
        render_fn: impl ComponentFunction<SuspenseBoundaryProps, M>,
    ) -> VComponent {
        let component_name = std::any::type_name_of_val(&render_fn);
        VComponent::new(
            move |wrapper: Self| render_fn.rebuild(wrapper.inner),
            self,
            component_name,
        )
    }
}
impl Properties for SuspenseBoundaryPropsWithOwner {
    type Builder = ();
    fn builder() -> Self::Builder {
        unreachable!()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        self.inner.memoize(&new.inner)
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children: SuspenseBoundaryPropsBuilder_Optional<Element>>
    SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)>
{
    pub fn build(self) -> SuspenseBoundaryPropsWithOwner {
        let (fallback, children) = self.fields;
        let fallback = fallback.0;
        let children = SuspenseBoundaryPropsBuilder_Optional::into_value(children, VNode::empty);
        SuspenseBoundaryPropsWithOwner {
            inner: SuspenseBoundaryProps { fallback, children },
            owner: self.owner,
        }
    }
}
#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialEq for SuspenseBoundaryProps {
    #[inline]
    fn eq(&self, other: &SuspenseBoundaryProps) -> bool {
        self.fallback == other.fallback && self.children == other.children
    }
}

/// Suspense Boundaries let you render a fallback UI while a child component is suspended.
///
/// # Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # fn Article() -> Element { rsx! { "Article" } }
/// fn App() -> Element {
///     rsx! {
///         SuspenseBoundary {
///             fallback: |context: SuspenseContext| rsx! {
///                 if let Some(placeholder) = context.suspense_placeholder() {
///                     {placeholder}
///                 } else {
///                     "Loading..."
///                 }
///             },
///             Article {}
///         }
///     }
/// }
/// ```
#[allow(non_snake_case)]
pub fn SuspenseBoundary(mut __props: SuspenseBoundaryProps) -> Element {
    unreachable!("SuspenseBoundary should not be called directly")
}
#[allow(non_snake_case)]
#[doc(hidden)]
mod SuspenseBoundary_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        SuspenseBoundary {},
    }
}
use generational_box::Owner;
#[allow(unused)]
pub use SuspenseBoundary_completions::Component::SuspenseBoundary;

/// Suspense has a custom diffing algorithm that diffs the suspended nodes in the background without rendering them
impl SuspenseBoundaryProps {
    /// Try to downcast [`AnyProps`] to [`SuspenseBoundaryProps`]
    pub(crate) fn downcast_from_props(props: &mut dyn AnyProps) -> Option<&mut Self> {
        let inner: Option<&mut SuspenseBoundaryPropsWithOwner> = props.props_mut().downcast_mut();
        inner.map(|inner| &mut inner.inner)
    }
}

impl SuspenseContext {
    /// Run a closure under a suspense boundary
    pub(crate) fn under_suspense_boundary<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::UnderSuspense(self.clone()), f)
    }

    /// Run a closure under a suspense placeholder
    pub(crate) fn in_suspense_placeholder<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::InSuspensePlaceholder(self.clone()), f)
    }

    /// Try to get a suspense boundary from a scope id
    pub fn downcast_suspense_boundary_from_scope(
        runtime: &Runtime,
        scope_id: ScopeId,
    ) -> Option<Self> {
        runtime
            .get_state(scope_id)
            .and_then(|scope| scope.suspense_boundary())
    }
}
