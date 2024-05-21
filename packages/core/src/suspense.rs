use crate::innerlude::*;
use std::{
    cell::{Ref, RefCell},
    fmt::Debug,
    rc::Rc,
};

/// A task that has been suspended which may have an optional loading placeholder
#[derive(Clone, PartialEq, Debug)]
pub struct SuspendedFuture {
    task: Task,
    pub(crate) placeholder: VNode,
}

impl SuspendedFuture {
    /// Create a new suspended future
    pub fn new(task: Task) -> Self {
        Self {
            task,
            placeholder: VNode::placeholder(),
        }
    }

    /// Get a placeholder to display while the future is suspended
    pub fn suspense_placeholder(&self) -> Option<VNode> {
        if self.placeholder == VNode::placeholder() {
            None
        } else {
            Some(self.placeholder.clone())
        }
    }

    /// Set a new placeholder the SuspenseBoundary may use to display while the future is suspended
    pub fn with_placeholder(mut self, placeholder: VNode) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Get the task that was suspended
    pub fn task(&self) -> Task {
        self.task
    }

    /// Clone the future while retaining the mounted information of the future
    pub(crate) fn clone_mounted(&self) -> Self {
        Self {
            task: self.task,
            placeholder: self.placeholder.clone_mounted(),
        }
    }
}

/// Provide an error boundary to catch errors from child components
pub fn use_suspense_boundary() -> SuspenseBoundary {
    use_hook(|| provide_context(SuspenseBoundary::new()))
}

/// A boundary that will capture any errors from child components
#[derive(Debug, Clone, Default)]
pub struct SuspenseBoundary {
    inner: Rc<SuspenseBoundaryInner>,
}

impl SuspenseBoundary {
    /// Create a new suspense boundary
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a suspended task
    pub(crate) fn add_suspended_task(&self, task: SuspendedFuture) {
        self.inner.suspended_tasks.borrow_mut().push(task);
        self.inner.id.needs_update();
    }

    /// Remove a suspended task
    pub(crate) fn remove_suspended_task(&self, task: Task) {
        self.inner
            .suspended_tasks
            .borrow_mut()
            .retain(|t| t.task != task);
        self.inner.id.needs_update();
    }

    /// Get all suspended tasks
    pub fn suspended_futures(&self) -> Ref<[SuspendedFuture]> {
        Ref::map(self.inner.suspended_tasks.borrow(), |tasks| {
            tasks.as_slice()
        })
    }

    /// Get the first suspended task with a loading placeholder
    pub fn suspense_placeholder(&self) -> Option<VNode> {
        self.inner
            .suspended_tasks
            .borrow()
            .iter()
            .find_map(|task| task.suspense_placeholder())
    }
}

/// A boundary that will capture any errors from child components
#[derive(Debug)]
pub struct SuspenseBoundaryInner {
    suspended_tasks: RefCell<Vec<SuspendedFuture>>,
    id: ScopeId,
}

impl Default for SuspenseBoundaryInner {
    fn default() -> Self {
        Self {
            suspended_tasks: RefCell::new(Vec::new()),
            id: current_scope_id().expect("to be in a dioxus runtime"),
        }
    }
}

/// Provides context methods to [`Result<T, RenderError>`] to show loading indicators
///
/// This trait is sealed and cannot be implemented outside of dioxus-core
pub trait SuspenseContext<T>: private::Sealed {
    /// Add a loading indicator if the result is suspended
    fn with_loading_placeholder(
        self,
        display_placeholder: impl FnOnce() -> Element,
    ) -> std::result::Result<T, RenderError>;
}

impl<T> SuspenseContext<T> for std::result::Result<T, RenderError> {
    fn with_loading_placeholder(
        self,
        display_placeholder: impl FnOnce() -> Element,
    ) -> std::result::Result<T, RenderError> {
        if let Err(RenderError::Suspended(suspense)) = self {
            Err(RenderError::Suspended(suspense.with_placeholder(
                display_placeholder().unwrap_or_default(),
            )))
        } else {
            self
        }
    }
}

pub(crate) mod private {
    use super::*;

    pub trait Sealed {}

    impl<T> Sealed for std::result::Result<T, RenderError> {}
}

#[doc = "Properties for the [`HiddenSSR`] component."]
#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone)]
pub struct HiddenSSRProps {
    pub(crate) render_in_ssr: bool,
    children: Element,
}
impl HiddenSSRProps {
    /**
    Create a builder for building `HiddenSSRProps`.
    On the builder, call `.render_in_ssr(...)`(optional), `.children(...)`(optional) to set the values of the fields.
    Finally, call `.build()` to create the instance of `HiddenSSRProps`.
                        */
    #[allow(dead_code, clippy::type_complexity)]
    pub fn builder() -> HiddenSSRPropsBuilder<((), ())> {
        HiddenSSRPropsBuilder {
            fields: ((), ()),
            _phantom: ::core::default::Default::default(),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct HiddenSSRPropsBuilder<TypedBuilderFields> {
    fields: TypedBuilderFields,
    _phantom: (),
}
impl Properties for HiddenSSRProps
where
    Self: Clone,
{
    type Builder = HiddenSSRPropsBuilder<((), ())>;
    fn builder() -> Self::Builder {
        HiddenSSRProps::builder()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        if !equal {
            let new_clone = new.clone();
            self.render_in_ssr = new_clone.render_in_ssr;
            self.children = new_clone.children;
        }
        equal
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait HiddenSSRPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> HiddenSSRPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> HiddenSSRPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> HiddenSSRPropsBuilder<((), __children)> {
    #[allow(clippy::type_complexity)]
    pub fn render_in_ssr(
        self,
        render_in_ssr: bool,
    ) -> HiddenSSRPropsBuilder<((bool,), __children)> {
        let render_in_ssr = (render_in_ssr,);
        let (_, children) = self.fields;
        HiddenSSRPropsBuilder {
            fields: (render_in_ssr, children),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum HiddenSSRPropsBuilder_Error_Repeated_field_render_in_ssr {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> HiddenSSRPropsBuilder<((bool,), __children)> {
    #[deprecated(note = "Repeated field render_in_ssr")]
    #[allow(clippy::type_complexity)]
    pub fn render_in_ssr(
        self,
        _: HiddenSSRPropsBuilder_Error_Repeated_field_render_in_ssr,
    ) -> HiddenSSRPropsBuilder<((bool,), __children)> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__render_in_ssr> HiddenSSRPropsBuilder<(__render_in_ssr, ())> {
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        children: Element,
    ) -> HiddenSSRPropsBuilder<(__render_in_ssr, (Element,))> {
        let children = (children,);
        let (render_in_ssr, _) = self.fields;
        HiddenSSRPropsBuilder {
            fields: (render_in_ssr, children),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum HiddenSSRPropsBuilder_Error_Repeated_field_children {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__render_in_ssr> HiddenSSRPropsBuilder<(__render_in_ssr, (Element,))> {
    #[deprecated(note = "Repeated field children")]
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        _: HiddenSSRPropsBuilder_Error_Repeated_field_children,
    ) -> HiddenSSRPropsBuilder<(__render_in_ssr, (Element,))> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<
        __children: HiddenSSRPropsBuilder_Optional<Element>,
        __render_in_ssr: HiddenSSRPropsBuilder_Optional<bool>,
    > HiddenSSRPropsBuilder<(__render_in_ssr, __children)>
{
    pub fn build(self) -> HiddenSSRProps {
        let (render_in_ssr, children) = self.fields;
        let render_in_ssr = HiddenSSRPropsBuilder_Optional::into_value(render_in_ssr, || false);
        let children = HiddenSSRPropsBuilder_Optional::into_value(children, || {
            Element::Ok(VNode::placeholder())
        });
        HiddenSSRProps {
            render_in_ssr,
            children,
        }
    }
}
/// A special component that is not rendered during SSR
#[allow(non_snake_case)]
#[doc(hidden)]
pub fn HiddenSSR(mut __props: HiddenSSRProps) -> Element {
    __props.children
}
