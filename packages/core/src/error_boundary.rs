use crate::{
    global_context::{current_scope_id, try_consume_context},
    innerlude::provide_context,
    use_hook, Element, IntoDynNode, Properties, ScopeId, Template, TemplateAttribute, TemplateNode,
    VNode,
};
use std::{
    any::{Any, TypeId},
    backtrace::Backtrace,
    cell::RefCell,
    error::Error,
    fmt::{Debug, Display},
    rc::Rc,
};

/// Provide an error boundary to catch errors from child components
pub fn use_error_boundary() -> ErrorBoundary {
    use_hook(|| provide_context(ErrorBoundary::new()))
}

/// A boundary that will capture any errors from child components
#[derive(Debug, Clone, Default)]
pub struct ErrorBoundary {
    inner: Rc<ErrorBoundaryInner>,
}

/// A boundary that will capture any errors from child components
pub struct ErrorBoundaryInner {
    error: RefCell<Option<CapturedError>>,
    _id: ScopeId,
}

impl Debug for ErrorBoundaryInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorBoundaryInner")
            .field("error", &self.error)
            .finish()
    }
}

/// A trait for any type that can be downcast to a concrete type and implements Debug. This is automatically implemented for all types that implement Any + Debug.
pub trait AnyDebug: Any + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Debug> AnyDebug for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
/// An instance of an error captured by a descendant component.
pub struct CapturedError {
    /// The error captured by the error boundary
    pub error: Box<dyn AnyDebug + 'static>,

    /// The backtrace of the error
    pub backtrace: Backtrace,

    /// The scope that threw the error
    pub scope: ScopeId,
}

impl Display for CapturedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Encountered error: {:?}\nIn scope: {:?}\nBacktrace: {}",
            self.error, self.scope, self.backtrace
        ))
    }
}

impl Error for CapturedError {}

impl CapturedError {
    /// Downcast the error type into a concrete error type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        if TypeId::of::<T>() == self.error.type_id() {
            self.error.as_any().downcast_ref::<T>()
        } else {
            None
        }
    }
}

impl Default for ErrorBoundaryInner {
    fn default() -> Self {
        Self {
            error: RefCell::new(None),
            _id: current_scope_id()
                .expect("Cannot create an error boundary outside of a component's scope."),
        }
    }
}

impl ErrorBoundary {
    /// Create a new error boundary
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new error boundary in the current scope
    pub(crate) fn new_in_scope(scope: ScopeId) -> Self {
        Self {
            inner: Rc::new(ErrorBoundaryInner {
                error: RefCell::new(None),
                _id: scope,
            }),
        }
    }

    /// Push an error into this Error Boundary
    pub fn insert_error(&self, scope: ScopeId, error: impl Debug + 'static, backtrace: Backtrace) {
        self.inner.error.replace(Some(CapturedError {
            error: Box::new(error),
            scope,
            backtrace,
        }));
        if self.inner._id != ScopeId::ROOT {
            self.inner._id.needs_update();
        }
    }

    /// Take any error that has been captured by this error boundary
    pub fn take_error(&self) -> Option<CapturedError> {
        self.inner.error.take()
    }
}

/// A trait to allow results to be thrown upwards to the nearest Error Boundary
///
/// The canonical way of using this trait is to throw results from hooks, aborting rendering
/// through question mark syntax. The throw method returns an option that evaluates to None
/// if there is an error, injecting the error to the nearest error boundary.
///
/// If the value is `Ok`, then throw returns the value, not aborting the rendering process.
///
/// The call stack is saved for this component and provided to the error boundary
///
/// ```rust, ignore
/// #[component]
/// fn app(count: String) -> Element {
///     let id: i32 = count.parse().throw()?;
///
///     rsx! {
///         div { "Count {}" }
///     }
/// }
/// ```
pub trait Throw<S = ()>: Sized {
    /// The value that will be returned in if the given value is `Ok`.
    type Out;

    /// Returns an option that evaluates to None if there is an error, injecting the error to the nearest error boundary.
    ///
    /// If the value is `Ok`, then throw returns the value, not aborting the rendering process.
    ///
    /// The call stack is saved for this component and provided to the error boundary
    ///
    ///
    /// Note that you can also manually throw errors using the throw method on `ScopeState` directly,
    /// which is what this trait shells out to.
    ///
    ///
    /// ```rust, ignore
    /// #[component]
    /// fn app( count: String) -> Element {
    ///     let id: i32 = count.parse().throw()?;
    ///
    ///     rsx! {
    ///         div { "Count {}" }
    ///     }
    /// }
    /// ```
    fn throw(self) -> Option<Self::Out>;

    /// Returns an option that evaluates to None if there is an error, injecting the error to the nearest error boundary.
    ///
    /// If the value is `Ok`, then throw returns the value, not aborting the rendering process.
    ///
    /// The call stack is saved for this component and provided to the error boundary
    ///
    ///
    /// Note that you can also manually throw errors using the throw method on `ScopeState` directly,
    /// which is what this trait shells out to.
    ///
    ///
    /// ```rust, ignore
    /// #[component]
    /// fn app( count: String) -> Element {
    ///     let id: i32 = count.parse().throw()?;
    ///
    ///     rsx! {
    ///         div { "Count {}" }
    ///     }
    /// }
    /// ```
    fn throw_with<D: Debug + 'static>(self, e: impl FnOnce() -> D) -> Option<Self::Out> {
        self.throw().or_else(|| throw_error(e()))
    }
}

fn throw_error<T>(e: impl Debug + 'static) -> Option<T> {
    if let Some(cx) = try_consume_context::<ErrorBoundary>() {
        match current_scope_id() {
            Some(id) => cx.insert_error(id, Box::new(e), Backtrace::capture()),
            None => {
                tracing::error!("Cannot throw error outside of a component's scope.")
            }
        }
    }

    None
}

/// We call clone on any errors that can be owned out of a reference
impl<'a, T, O: Debug + 'static, E: ToOwned<Owned = O>> Throw for &'a Result<T, E> {
    type Out = &'a T;

    fn throw(self) -> Option<Self::Out> {
        match self {
            Ok(t) => Some(t),
            Err(e) => throw_error(e.to_owned()),
        }
    }

    fn throw_with<D: Debug + 'static>(self, err: impl FnOnce() -> D) -> Option<Self::Out> {
        match self {
            Ok(t) => Some(t),
            Err(_e) => throw_error(err()),
        }
    }
}

/// Or just throw errors we know about
impl<T, E: Debug + 'static> Throw for Result<T, E> {
    type Out = T;

    fn throw(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => throw_error(e),
        }
    }

    fn throw_with<D: Debug + 'static>(self, error: impl FnOnce() -> D) -> Option<Self::Out> {
        self.ok().or_else(|| throw_error(error()))
    }
}

/// Or just throw errors we know about
impl<T> Throw for Option<T> {
    type Out = T;

    fn throw(self) -> Option<T> {
        self.or_else(|| throw_error("Attempted to unwrap a None value."))
    }

    fn throw_with<D: Debug + 'static>(self, error: impl FnOnce() -> D) -> Option<Self::Out> {
        self.or_else(|| throw_error(error()))
    }
}

#[derive(Clone)]
pub struct ErrorHandler(Rc<dyn Fn(CapturedError) -> Element>);
impl<F: Fn(CapturedError) -> Element + 'static> From<F> for ErrorHandler {
    fn from(value: F) -> Self {
        Self(Rc::new(value))
    }
}
fn default_handler(error: CapturedError) -> Element {
    static TEMPLATE: Template = Template {
        name: "error_handle.rs:42:5:884",
        roots: &[TemplateNode::Element {
            tag: "pre",
            namespace: None,
            attrs: &[TemplateAttribute::Static {
                name: "color",
                namespace: Some("style"),
                value: "red",
            }],
            children: &[TemplateNode::DynamicText { id: 0usize }],
        }],
        node_paths: &[&[0u8, 0u8]],
        attr_paths: &[],
    };
    Some(VNode::new(
        None,
        TEMPLATE,
        Box::new([error.to_string().into_dyn_node()]),
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
    fn memoize(&mut self, _: &Self) -> bool {
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
        let children = ErrorBoundaryPropsBuilder_Optional::into_value(children, || {
            ::core::default::Default::default()
        });
        let handle_error = ErrorBoundaryPropsBuilder_Optional::into_value(handle_error, || {
            ErrorHandler(Rc::new(default_handler))
        });
        ErrorBoundaryProps {
            children,
            handle_error,
        }
    }
}
/// Create a new error boundary component.
///
/// ## Details
///
/// Error boundaries handle errors within a specific part of your application. Any errors passed in a child with [`Throw`] will be caught by the nearest error boundary.
///
/// ## Example
///
/// ```rust, ignore
/// rsx!{
///     ErrorBoundary {
///         handle_error: |error| rsx! { "Oops, we encountered an error. Please report {error} to the developer of this application" }
///         ThrowsError {}
///     }
/// }
/// ```
///
/// ## Usage
///
/// Error boundaries are an easy way to handle errors in your application.
/// They are similar to `try/catch` in JavaScript, but they only catch errors in the tree below them.
/// Error boundaries are quick to implement, but it can be useful to individually handle errors in your components to provide a better user experience when you know that an error is likely to occur.
#[allow(non_upper_case_globals, non_snake_case)]
pub fn ErrorBoundary(props: ErrorBoundaryProps) -> Element {
    let error_boundary = use_error_boundary();
    match error_boundary.take_error() {
        Some(error) => (props.handle_error.0)(error),
        None => Some({
            static TEMPLATE: Template = Template {
                name: "examples/error_handle.rs:81:17:2342",
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
        }),
    }
}
