use crate::{
    global_context::current_scope_id, innerlude::provide_context, use_hook, Element, IntoDynNode,
    Properties, ScopeId, Template, TemplateAttribute, TemplateNode, VNode,
};
use std::{
    any::{Any, TypeId},
    backtrace::Backtrace,
    cell::{Ref, RefCell},
    error::Error,
    fmt::{Debug, Display},
    rc::Rc,
};

/// A panic in a component that was caught by an error boundary.
///
/// <div class="warning">
///
/// WASM currently does not support caching unwinds, so this struct will not be created in WASM.
///
/// </div>
pub struct CapturedPanic {
    #[allow(dead_code)]
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
pub(crate) struct ErrorBoundaryInner {
    errors: RefCell<Vec<CapturedError>>,
    id: ScopeId,
}

impl Debug for ErrorBoundaryInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorBoundaryInner")
            .field("error", &self.errors)
            .finish()
    }
}

/// A trait for any type that can be downcast to a concrete type and implements Debug. This is automatically implemented for all types that implement Any + Debug.
pub trait AnyError {
    fn as_any(&self) -> &dyn Any;
    fn as_error(&self) -> &dyn Error;
}

/// An wrapper error type for types that only implement Display. We use a inner type here to avoid overlapping implementations for DisplayError and impl Error
struct DisplayError(DisplayErrorInner);

impl<E: Display + 'static> From<E> for DisplayError {
    fn from(e: E) -> Self {
        Self(DisplayErrorInner(Box::new(e)))
    }
}

struct DisplayErrorInner(Box<dyn Display>);
impl Display for DisplayErrorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Debug for DisplayErrorInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for DisplayErrorInner {}

impl AnyError for DisplayError {
    fn as_any(&self) -> &dyn Any {
        &self.0 .0
    }

    fn as_error(&self) -> &dyn Error {
        &self.0
    }
}

/// Provides context methods to [`Result`] and [`Option`] types that are compatible with [`CapturedError`]
///
/// This trait is sealed and cannot be implemented outside of dioxus-core
pub trait Context<T, E>: private::Sealed {
    /// Add a visual representation of the error that the [`ErrorBoundary`] may render
    fn show(self, display_error: impl FnOnce(&E) -> Element) -> Result<T>;

    /// Wrap the result with context that is lazily evaluated if an error occurs and return a new boxed error
    fn context<C: Display + 'static>(self, context: C) -> Result<T>;

    /// Wrap the result with context that is lazily evaluated if an error occurs and return a new boxed error
    fn with_context<C: Display + 'static>(self, context: impl FnOnce() -> C) -> Result<T>;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
where
    E: Error + 'static,
{
    fn show(self, display_error: impl FnOnce(&E) -> Element) -> Result<T> {
        // We don't use result mapping to avoid extra frames
        match self {
            std::result::Result::Ok(value) => Ok(value),
            Err(error) => {
                let render = display_error(&error).unwrap_or_default();
                let mut error: CapturedError = error.into();
                error.render = render;
                Err(error)
            }
        }
    }

    fn context<C: Display + 'static>(self, context: C) -> Result<T> {
        self.with_context(|| context)
    }

    fn with_context<C: Display + 'static>(self, context: impl FnOnce() -> C) -> Result<T> {
        // We don't use result mapping to avoid extra frames
        match self {
            std::result::Result::Ok(value) => Ok(value),
            Err(error) => {
                let mut error: CapturedError = error.into();
                error.context.push(Rc::new(ErrorContext {
                    backtrace: Backtrace::capture(),
                    context: Box::new(context()),
                    scope: current_scope_id(),
                }));
                Err(error)
            }
        }
    }
}

impl<T> Context<T, CapturedError> for Option<T> {
    fn show(self, display_error: impl FnOnce(&CapturedError) -> Element) -> Result<T> {
        // We don't use result mapping to avoid extra frames
        match self {
            Some(value) => Ok(value),
            None => {
                let mut error = CapturedError::from_display("Value was none");
                let render = display_error(&error).unwrap_or_default();
                error.render = render;
                Err(error)
            }
        }
    }

    fn context<C: Display + 'static>(self, context: C) -> Result<T> {
        self.with_context(|| context)
    }

    fn with_context<C: Display + 'static>(self, context: impl FnOnce() -> C) -> Result<T> {
        // We don't use result mapping to avoid extra frames
        match self {
            Some(value) => Ok(value),
            None => {
                let error = CapturedError::from_display(context());
                Err(error)
            }
        }
    }
}

pub(crate) mod private {
    use super::*;

    pub trait Sealed {}

    impl<T, E> Sealed for std::result::Result<T, E> where E: Error {}
    impl<T> Sealed for Option<T> {}
}

impl<T: Any + Error> AnyError for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_error(&self) -> &dyn Error {
        self
    }
}

/// Errors can have additional context added as they bubble up the render tree
/// This context can be used to provide additional information to the user
struct ErrorContext {
    backtrace: Backtrace,
    context: Box<dyn Display>,
    scope: Option<ScopeId>,
}

impl Debug for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorContext")
            .field("backtrace", &self.backtrace)
            .field("context", &self.context.to_string())
            .finish()
    }
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ErrorContext {
            backtrace,
            context,
            scope,
        } = self;

        write!(f, "{context} (from ")?;

        if let Some(scope) = scope {
            write!(f, "scope {scope:?} ")?;
        }

        write!(f, "at {backtrace:?})")
    }
}

/// A type alias for a result that can be either a boxed error or a value
/// This is useful to avoid having to use `Result<T, CapturedError>` everywhere
pub type Result<T = ()> = std::result::Result<T, CapturedError>;

/// A helper function for an Ok result that can be either a boxed error or a value
/// This is useful to avoid having to use `Ok<T, CapturedError>` everywhere
#[allow(non_snake_case)]
pub fn Ok<T>(value: T) -> Result<T> {
    Result::Ok(value)
}

#[derive(Clone)]
/// An instance of an error captured by a descendant component.
pub struct CapturedError {
    /// The error captured by the error boundary
    error: Rc<dyn AnyError + 'static>,

    /// The backtrace of the error
    backtrace: Rc<Backtrace>,

    /// The scope that threw the error
    scope: ScopeId,

    /// An error message that can be displayed to the user
    pub(crate) render: VNode,

    /// Additional context that was added to the error
    context: Vec<Rc<ErrorContext>>,
}

impl Debug for CapturedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedError")
            .field("error", &self.error.as_error())
            .field("backtrace", &self.backtrace)
            .field("scope", &self.scope)
            .finish()
    }
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
    /// Create a new captured error
    pub fn new(error: impl AnyError + 'static) -> Self {
        Self {
            error: Rc::new(error),
            backtrace: Rc::new(Backtrace::capture()),
            scope: current_scope_id().unwrap_or(ScopeId::ROOT),
            render: Default::default(),
            context: Default::default(),
        }
    }

    /// Create a new error from a type that only implements [`Display`]. If your type implements [`Error`], you can use [`CapturedError::from`] instead.
    pub fn from_display(error: impl Display + 'static) -> Self {
        Self {
            error: Rc::new(DisplayError::from(error)),
            backtrace: Rc::new(Backtrace::capture()),
            scope: current_scope_id().unwrap_or(ScopeId::ROOT),
            render: Default::default(),
            context: Default::default(),
        }
    }

    /// Mark the error as being thrown from a specific scope
    pub fn with_origin(mut self, scope: ScopeId) -> Self {
        self.scope = scope;
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

    /// Get a VNode representation of the error if the error provides one
    pub fn visualize(&self) -> Option<Element> {
        if self.render == VNode::placeholder() {
            None
        } else {
            Some(std::result::Result::Ok(self.render.clone()))
        }
    }
}

impl PartialEq for CapturedError {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

impl Display for CapturedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Encountered error: {:?}\nIn scope: {:?}\nBacktrace: {}\nContext: ",
            self.error.as_error(),
            self.scope,
            self.backtrace
        ))?;
        for context in &*self.context {
            f.write_fmt(format_args!("{}\n", context))?;
        }
        std::result::Result::Ok(())
    }
}

impl CapturedError {
    /// Downcast the error type into a concrete error type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        if TypeId::of::<T>() == (*self.error).type_id() {
            self.error.as_any().downcast_ref::<T>()
        } else {
            None
        }
    }
}

impl Default for ErrorBoundaryInner {
    fn default() -> Self {
        Self {
            errors: RefCell::new(Vec::new()),
            id: current_scope_id()
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
                errors: RefCell::new(Vec::new()),
                id: scope,
            }),
        }
    }

    /// Push an error into this Error Boundary
    pub fn insert_error(&self, error: CapturedError) {
        self.inner.errors.borrow_mut().push(error);
        if self.inner.id != ScopeId::ROOT {
            self.inner.id.needs_update();
        }
    }

    /// Clear all errors from this Error Boundary
    pub fn clear_errors(&self) {
        self.inner.errors.borrow_mut().clear();
    }

    /// Return any errors that have been captured by this error boundary
    pub fn errors(&self) -> Ref<[CapturedError]> {
        Ref::map(self.inner.errors.borrow(), |errors| errors.as_slice())
    }
}

pub(crate) fn throw_into(error: impl Into<CapturedError>, scope: ScopeId) {
    let error = error.into();
    if let Some(cx) = scope.consume_context::<ErrorBoundary>() {
        cx.insert_error(error)
    } else {
        tracing::error!(
            "Tried to throw an error into an error boundary, but failed to locate a boundary: {:?}",
            error
        )
    }
}

#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct ErrorHandler(Rc<dyn Fn(&[CapturedError]) -> Element>);
impl<F: Fn(&[CapturedError]) -> Element + 'static> From<F> for ErrorHandler {
    fn from(value: F) -> Self {
        Self(Rc::new(value))
    }
}

fn default_handler(errors: &[CapturedError]) -> Element {
    static TEMPLATE: Template = Template {
        name: "error_handle.rs:42:5:884",
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
            .iter()
            .map(|e| {
                static TEMPLATE: Template = Template {
                    name: "error_handle.rs:43:5:884",
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
        let children = ErrorBoundaryPropsBuilder_Optional::into_value(children, || {
            std::result::Result::Ok(VNode::placeholder())
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
/// Create a new error boundary component that catches any errors thrown from child components
///
/// ## Details
///
/// Error boundaries handle errors within a specific part of your application. Any errors passed in a child with [`Throw`] will be caught by the nearest error boundary.
///
/// ## Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # fn ThrowsError() -> Element { todo!() }
/// rsx! {
///     ErrorBoundary {
///         handle_error: |errors: &[CapturedError]| rsx! { "Oops, we encountered an error. Please report {errors[0]} to the developer of this application" },
///         ThrowsError {}
///     }
/// };
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
    let errors = error_boundary.errors();
    match &*errors {
        [] => std::result::Result::Ok({
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
        errors => (props.handle_error.0)(errors),
    }
}
