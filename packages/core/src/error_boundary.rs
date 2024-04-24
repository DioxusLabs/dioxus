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

/// A panic in a component that was caught by an error boundary.
///
/// NOTE: WASM currently does not support caching unwinds, so this struct will not be created in WASM.
pub struct CapturedPanic {
    /// The error that was caught
    pub error: Box<dyn Any + 'static>,
}

impl Debug for CapturedPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedPanic").finish()
    }
}

impl Display for CapturedPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Encountered panic: {:?}", self.error))
    }
}

impl Error for CapturedPanic {}

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
pub trait AnyError: Any + Error {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Error> AnyError for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct Context {
    backtrace: Backtrace,
    context: String,
}

#[derive(Debug, Clone)]
/// An instance of an error captured by a descendant component.
pub struct CapturedError {
    /// The error captured by the error boundary
    error: Rc<dyn AnyError + 'static>,

    /// The backtrace of the error
    backtrace: Rc<Backtrace>,

    /// The scope that threw the error
    scope: ScopeId,

    /// An error message that can be displayed to the user
    render: VNode,

    /// Additional context that was added to the error
    context: Vec<Rc<Context>>,
}

impl<E: AnyError + 'static> From<E> for CapturedError {
    fn from(error: E) -> Self {
        Self {
            error: Rc::new(error),
            backtrace: Rc::new(Backtrace::capture()),
            scope: current_scope_id()
                .expect("Cannot create an error boundary outside of a component's scope."),
            render: Default::default(),
            context: Default::default(),
        }
    }
}

impl CapturedError {
    pub fn new(error: impl AnyError + 'static, scope: ScopeId) -> Self {
        Self {
            error: Rc::new(error),
            backtrace: Rc::new(Backtrace::capture()),
            scope,
            render: Default::default(),
            context: Default::default(),
        }
    }

    pub fn with_context<T: Display>(mut self, context: T) -> Self {
        self.context.push(Rc::new(Context {
            backtrace: Backtrace::capture(),
            context: format!("{context}"),
        }));
        self
    }

    /// Clone the error while retaining the mounted information of the error
    pub(crate) fn clone_mounted(&self) -> Self {
        Self {
            error: self.error.clone(),
            backtrace: self.backtrace.clone(),
            scope: self.scope,
            render: self.render.clone_mounted(),
            context: self.context.clone(),
        }
    }
}

impl PartialEq for CapturedError {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self.error) == format!("{:?}", other.error)
    }
}

impl Display for CapturedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Encountered error: {:?}\nIn scope: {:?}\nBacktrace: {}\nContext: ",
            self.error, self.scope, self.backtrace
        ))?;
        for context in &*self.context {
            f.write_fmt(format_args!("{:?}\n", context))?;
        }
        Ok(())
    }
}

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
    pub fn insert_error(&self, error: CapturedError) {
        self.inner.error.replace(Some(error));
        if self.inner._id != ScopeId::ROOT {
            self.inner._id.needs_update();
        }
    }

    /// Take any error that has been captured by this error boundary
    pub fn take_error(&self) -> Option<CapturedError> {
        self.inner.error.take()
    }
}

pub(crate) fn throw_error(error: CapturedError) {
    if let Some(cx) = try_consume_context::<ErrorBoundary>() {
        cx.insert_error(error)
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
    Ok(VNode::new(
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
        let children =
            ErrorBoundaryPropsBuilder_Optional::into_value(children, || Ok(Default::default()));
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
        None => Ok({
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
