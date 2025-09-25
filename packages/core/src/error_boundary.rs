use crate::{
    global_context::current_scope_id,
    innerlude::{provide_context, CapturedError},
    use_hook, Element, IntoDynNode, Properties, ScopeId, Template, TemplateAttribute, TemplateNode,
    VNode,
};
use std::{
    any::Any,
    cell::{Ref, RefCell},
    fmt::{Debug, Display},
    rc::Rc,
};

/// Return early with an error.
///
/// This macro is equivalent to
/// <code>return Err([anyhow!($args\...)][anyhow!])</code>.
///
/// The surrounding function's or closure's return value is required to be
/// <code>Result&lt;_, [anyhow::Error][crate::Error]&gt;</code>.
///
/// [anyhow!]: crate::anyhow
///
/// # Example
///
/// ```
/// # use anyhow::{bail, Result};
/// #
/// # fn has_permission(user: usize, resource: usize) -> bool {
/// #     true
/// # }
/// #
/// # fn main() -> Result<()> {
/// #     let user = 0;
/// #     let resource = 0;
/// #
/// if !has_permission(user, resource) {
///     bail!("permission denied for accessing {}", resource);
/// }
/// #     Ok(())
/// # }
/// ```
///
/// ```
/// # use anyhow::{bail, Result};
/// # use thiserror::Error;
/// #
/// # const MAX_DEPTH: usize = 1;
/// #
/// #[derive(Error, Debug)]
/// enum ScienceError {
///     #[error("recursion limit exceeded")]
///     RecursionLimitExceeded,
///     # #[error("...")]
///     # More = (stringify! {
///     ...
///     # }, 1).1,
/// }
///
/// # fn main() -> Result<()> {
/// #     let depth = 0;
/// #
/// if depth > MAX_DEPTH {
///     bail!(ScienceError::RecursionLimitExceeded);
/// }
/// #     Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return $crate::internal::Err($crate::internal::__anyhow!($msg).into())
    };
    ($err:expr $(,)?) => {
        return $crate::internal::Err($crate::internal::__anyhow!($err).into())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return $crate::internal::Err($crate::internal::__anyhow!($fmt, $($arg)*).into())
    };
}

/// A panic in a component that was caught by an error boundary.
///
/// <div class="warning">
///
/// WASM currently does not support caching unwinds, so this struct will not be created in WASM.
///
/// </div>
pub(crate) struct CapturedPanic(pub(crate) Box<dyn Any + Send + 'static>);
unsafe impl Sync for CapturedPanic {}
impl std::error::Error for CapturedPanic {}
impl Debug for CapturedPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedPanic").finish()
    }
}

impl Display for CapturedPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Encountered panic: {:?}", self.0))
    }
}

/// Provide an error boundary to catch errors from child components
pub fn provide_error_boundary() -> ErrorContext {
    provide_context(ErrorContext::new(current_scope_id()))
}

/// A context with information about suspended components
#[derive(Debug, Clone)]
pub struct ErrorContext {
    errors: Rc<RefCell<Vec<CapturedError>>>,
    id: ScopeId,
}

impl PartialEq for ErrorContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.errors, &other.errors)
    }
}

impl ErrorContext {
    /// Create a new suspense boundary in a specific scope
    pub(crate) fn new(id: ScopeId) -> Self {
        Self {
            errors: Rc::new(RefCell::new(vec![])),
            id,
        }
    }

    /// Get all errors thrown from child components
    pub fn errors(&self) -> Ref<'_, [CapturedError]> {
        Ref::map(self.errors.borrow(), |errors| errors.as_slice())
    }

    /// Push an error into this Error Boundary
    pub fn insert_error(&self, error: CapturedError) {
        self.errors.borrow_mut().push(error);
        self.id.needs_update();
    }

    /// Clear all errors from this Error Boundary
    pub fn clear_errors(&self) {
        self.errors.borrow_mut().clear();
        self.id.needs_update();
    }
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct ErrorHandler(Rc<dyn Fn(ErrorContext) -> Element>);
impl<F: Fn(ErrorContext) -> Element + 'static> From<F> for ErrorHandler {
    fn from(value: F) -> Self {
        Self(Rc::new(value))
    }
}

fn default_handler(errors: ErrorContext) -> Element {
    static TEMPLATE: Template = Template {
        roots: &[TemplateNode::Element {
            tag: "div",
            namespace: None,
            attrs: &[TemplateAttribute::Static {
                name: "color",
                namespace: Some("style"),
                value: "red",
            }],
            children: &[TemplateNode::Dynamic { id: 0usize }],
        }],
        node_paths: &[&[0u8, 0u8]],
        attr_paths: &[],
    };
    std::result::Result::Ok(VNode::new(
        None,
        TEMPLATE,
        Box::new([errors
            .errors()
            .iter()
            .map(|e| {
                static TEMPLATE: Template = Template {
                    roots: &[TemplateNode::Element {
                        tag: "pre",
                        namespace: None,
                        attrs: &[],
                        children: &[TemplateNode::Dynamic { id: 0usize }],
                    }],
                    node_paths: &[&[0u8, 0u8]],
                    attr_paths: &[],
                };
                VNode::new(
                    None,
                    TEMPLATE,
                    Box::new([e.to_string().into_dyn_node()]),
                    Default::default(),
                )
            })
            .into_dyn_node()]),
        Default::default(),
    ))
}

#[derive(Clone)]
pub struct ErrorBoundaryProps {
    children: Element,
    handle_error: ErrorHandler,
}
impl ErrorBoundaryProps {
    /**
    Create a builder for building `ErrorBoundaryProps`.
    On the builder, call `.children(...)`(optional), `.handle_error(...)`(optional) to set the values of the fields.
    Finally, call `.build()` to create the instance of `ErrorBoundaryProps`.
                        */
    #[allow(dead_code)]
    pub fn builder() -> ErrorBoundaryPropsBuilder<((), ())> {
        ErrorBoundaryPropsBuilder { fields: ((), ()) }
    }
}

#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct ErrorBoundaryPropsBuilder<TypedBuilderFields> {
    fields: TypedBuilderFields,
}
impl<TypedBuilderFields> Clone for ErrorBoundaryPropsBuilder<TypedBuilderFields>
where
    TypedBuilderFields: Clone,
{
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
        }
    }
}
impl Properties for ErrorBoundaryProps {
    type Builder = ErrorBoundaryPropsBuilder<((), ())>;
    fn builder() -> Self::Builder {
        ErrorBoundaryProps::builder()
    }
    fn memoize(&mut self, other: &Self) -> bool {
        *self = other.clone();
        false
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait ErrorBoundaryPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> ErrorBoundaryPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> ErrorBoundaryPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__handle_error> ErrorBoundaryPropsBuilder<((), __handle_error)> {
    pub fn children(
        self,
        children: Element,
    ) -> ErrorBoundaryPropsBuilder<((Element,), __handle_error)> {
        let children = (children,);
        let (_, handle_error) = self.fields;
        ErrorBoundaryPropsBuilder {
            fields: (children, handle_error),
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum ErrorBoundaryPropsBuilder_Error_Repeated_field_children {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__handle_error> ErrorBoundaryPropsBuilder<((Element,), __handle_error)> {
    #[deprecated(note = "Repeated field children")]
    pub fn children(
        self,
        _: ErrorBoundaryPropsBuilder_Error_Repeated_field_children,
    ) -> ErrorBoundaryPropsBuilder<((Element,), __handle_error)> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> ErrorBoundaryPropsBuilder<(__children, ())> {
    pub fn handle_error(
        self,
        handle_error: impl ::core::convert::Into<ErrorHandler>,
    ) -> ErrorBoundaryPropsBuilder<(__children, (ErrorHandler,))> {
        let handle_error = (handle_error.into(),);
        let (children, _) = self.fields;
        ErrorBoundaryPropsBuilder {
            fields: (children, handle_error),
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum ErrorBoundaryPropsBuilder_Error_Repeated_field_handle_error {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> ErrorBoundaryPropsBuilder<(__children, (ErrorHandler,))> {
    #[deprecated(note = "Repeated field handle_error")]
    pub fn handle_error(
        self,
        _: ErrorBoundaryPropsBuilder_Error_Repeated_field_handle_error,
    ) -> ErrorBoundaryPropsBuilder<(__children, (ErrorHandler,))> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<
        __handle_error: ErrorBoundaryPropsBuilder_Optional<ErrorHandler>,
        __children: ErrorBoundaryPropsBuilder_Optional<Element>,
    > ErrorBoundaryPropsBuilder<(__children, __handle_error)>
{
    pub fn build(self) -> ErrorBoundaryProps {
        let (children, handle_error) = self.fields;
        let children = ErrorBoundaryPropsBuilder_Optional::into_value(children, VNode::empty);
        let handle_error = ErrorBoundaryPropsBuilder_Optional::into_value(handle_error, || {
            ErrorHandler(Rc::new(default_handler))
        });
        ErrorBoundaryProps {
            children,
            handle_error,
        }
    }
}

/// Create a new error boundary component that catches any errors thrown from child components
///
/// ## Details
///
/// Error boundaries handle errors within a specific part of your application. They are similar to `try/catch` in JavaScript, but they only catch errors in the tree below them.
/// Any errors passed up from a child will be caught by the nearest error boundary. Error boundaries are quick to implement, but it can be useful to individually handle errors
/// in your components to provide a better user experience when you know that an error is likely to occur.
///
/// ## Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// fn App() -> Element {
///     let mut multiplier = use_signal(|| String::from("2"));
///     rsx! {
///         input {
///             r#type: "text",
///             value: multiplier,
///             oninput: move |e| multiplier.set(e.value())
///         }
///         ErrorBoundary {
///             handle_error: |errors: ErrorContext| {
///                 rsx! {
///                     div {
///                         "Oops, we encountered an error. Please report {errors:?} to the developer of this application"
///                     }
///                 }
///             },
///             Counter {
///                 multiplier
///             }
///         }
///     }
/// }
///
/// #[component]
/// fn Counter(multiplier: ReadSignal<String>) -> Element {
///     let multiplier_parsed = multiplier().parse::<usize>()?;
///     let mut count = use_signal(|| multiplier_parsed);
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let multiplier_parsed = multiplier().parse::<usize>()?;
///                 *count.write() *= multiplier_parsed;
///                 Ok(())
///             },
///             "{count}x{multiplier}"
///         }
///     }
/// }
/// ```
///
/// ## Resetting the error boundary
///
/// Once the error boundary catches an error, it will render the rsx returned from the handle_error function instead of the children. To reset the error boundary,
/// you can call the [`ErrorContext::clear_errors`] method. This will clear all errors and re-render the children.
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn App() -> Element {
///     let mut multiplier = use_signal(|| String::new());
///     rsx! {
///         input {
///             r#type: "text",
///             value: multiplier,
///             oninput: move |e| multiplier.set(e.value())
///         }
///         ErrorBoundary {
///             handle_error: |errors: ErrorContext| {
///                 rsx! {
///                     div {
///                         "Oops, we encountered an error. Please report {errors:?} to the developer of this application"
///                     }
///                     button {
///                         onclick: move |_| {
///                             errors.clear_errors();
///                         },
///                         "try again"
///                     }
///                 }
///             },
///             Counter {
///                 multiplier
///             }
///         }
///     }
/// }
///
/// #[component]
/// fn Counter(multiplier: ReadSignal<String>) -> Element {
///     let multiplier_parsed = multiplier().parse::<usize>()?;
///     let mut count = use_signal(|| multiplier_parsed);
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let multiplier_parsed = multiplier().parse::<usize>()?;
///                 *count.write() *= multiplier_parsed;
///                 Ok(())
///             },
///             "{count}x{multiplier}"
///         }
///     }
/// }
/// ```
#[allow(non_upper_case_globals, non_snake_case)]
pub fn ErrorBoundary(props: ErrorBoundaryProps) -> Element {
    let error_boundary = use_hook(provide_error_boundary);
    let errors = error_boundary.errors();
    let has_errors = !errors.is_empty();
    // Drop errors before running user code that might borrow the error lock
    drop(errors);

    if has_errors {
        (props.handle_error.0)(error_boundary.clone())
    } else {
        std::result::Result::Ok({
            static TEMPLATE: Template = Template {
                roots: &[TemplateNode::Dynamic { id: 0usize }],
                node_paths: &[&[0u8]],
                attr_paths: &[],
            };
            VNode::new(
                None,
                TEMPLATE,
                Box::new([(props.children).into_dyn_node()]),
                Default::default(),
            )
        })
    }
}
