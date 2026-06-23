use crate::{
    DynamicValues, Element, IntoDynNode, ReactiveContext, Subscribers, Template, VNode,
    innerlude::{CapturedError, provide_context},
    try_consume_context, use_hook,
};
use dioxus_core_template::{TemplateRawTree, TemplateStorage};
use std::{
    any::Any,
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

static ERROR_DYNAMIC_TREE: TemplateRawTree = TemplateRawTree::DynamicNode;

/// Return early with an error.
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return ::std::result::Result::Err($crate::internal::__anyhow!($msg).into())
    };
    ($err:expr $(,)?) => {
        return ::std::result::Result::Err($crate::internal::__anyhow!($err).into())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return ::std::result::Result::Err($crate::internal::__anyhow!($fmt, $($arg)*).into())
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

/// A context supplied by fullstack to create hydration compatible error boundaries. Generally, this
/// is not present and the default in memory error boundary is used. If fullstack is enabled, it will
/// provide its own factory that handles syncing errors to the hydration context
#[derive(Clone, Copy)]
struct CreateErrorBoundary(fn() -> ErrorContext);

impl Default for CreateErrorBoundary {
    fn default() -> Self {
        Self(|| ErrorContext::new(None))
    }
}

/// Provides a method that is used to create error boundaries in `use_error_boundary_provider`.
/// This is only called from fullstack to create a hydration compatible error boundary
pub fn provide_create_error_boundary(create_error_boundary: fn() -> ErrorContext) {
    provide_context(CreateErrorBoundary(create_error_boundary));
}

/// Create an error boundary with the current error boundary factory (either hydration compatible or default)
fn create_error_boundary() -> ErrorContext {
    let create_error_boundary = try_consume_context::<CreateErrorBoundary>().unwrap_or_default();
    (create_error_boundary.0)()
}

/// Provide an error boundary to catch errors from child components. This needs to called in a hydration comptable
/// order if fullstack is enabled
pub(crate) fn use_error_boundary_provider() -> ErrorContext {
    use_hook(|| provide_context(create_error_boundary()))
}

/// A context with information about suspended components
#[derive(Clone)]
pub struct ErrorContext {
    error: Rc<RefCell<Option<CapturedError>>>,
    subscribers: Subscribers,
}

impl Debug for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorContext")
            .field("error", &self.error)
            .finish()
    }
}

impl PartialEq for ErrorContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.error, &other.error)
    }
}

impl ErrorContext {
    /// Create a new suspense boundary in a specific scope
    pub fn new(error: Option<CapturedError>) -> Self {
        Self {
            error: Rc::new(RefCell::new(error)),
            subscribers: Subscribers::new(),
        }
    }

    /// Get the current error, if any. If multiple components have errored, this will return the first
    /// error that made it to this boundary.
    pub fn error(&self) -> Option<CapturedError> {
        // Subscribe to the current reactive context if one exists. This is usually
        // the error boundary component that is rendering the errors
        if let Some(rc) = ReactiveContext::current() {
            self.subscribers.add(rc);
        }

        self.error.borrow().clone()
    }

    /// Push an error into this Error Boundary
    pub fn insert_error(&self, error: CapturedError) {
        self.error.borrow_mut().replace(error);
        self.mark_dirty()
    }

    /// Clear all errors from this Error Boundary
    pub fn clear_errors(&self) {
        self.error.borrow_mut().take();
        self.mark_dirty();
    }

    /// Mark the error context as dirty and notify all subscribers
    fn mark_dirty(&self) {
        let mut this_subscribers_vec = Vec::new();
        self.subscribers
            .visit(|subscriber| this_subscribers_vec.push(*subscriber));
        for subscriber in this_subscribers_vec {
            self.subscribers.remove(&subscriber);
            subscriber.mark_dirty();
        }
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
    static ATTRS: TemplateRawTree = TemplateRawTree::StaticAttr {
        name: "color",
        value: "red",
        namespace: Some("style"),
    };
    static TREE: TemplateRawTree = TemplateRawTree::Element {
        tag: "div",
        namespace: None,
        attrs: &ATTRS,
        children: &ERROR_DYNAMIC_TREE,
    };
    static STORAGE: TemplateStorage<8, 4, 2> = TemplateStorage::build_from_tree(&TREE);
    static TEMPLATE: Template = STORAGE.as_template();

    std::result::Result::Ok(VNode::new(
        TEMPLATE,
        DynamicValues::from_parts(
            None,
            Box::new([errors
                .error()
                .iter()
                .map(|e| {
                    static TREE: TemplateRawTree = TemplateRawTree::Element {
                        tag: "pre",
                        namespace: None,
                        attrs: &TemplateRawTree::Empty,
                        children: &ERROR_DYNAMIC_TREE,
                    };
                    static STORAGE: TemplateStorage<4, 1, 2> =
                        TemplateStorage::build_from_tree(&TREE);
                    static INNER_TEMPLATE: Template = STORAGE.as_template();

                    VNode::new(
                        INNER_TEMPLATE,
                        DynamicValues::from_parts(
                            None,
                            Box::new([e.to_string().into_dyn_node()]),
                            Box::new([]),
                        ),
                    )
                })
                .into_dyn_node()]),
            Box::new([]),
        ),
    ))
}

#[derive(dioxus_core_macro::Props, Clone)]
pub struct ErrorBoundaryProps {
    children: Element,
    #[props(into, default = ErrorHandler(Rc::new(default_handler)))]
    handle_error: ErrorHandler,
}

// The `Props` derive needs the props to be `PartialEq` for memoization, but `ErrorHandler` wraps an
// `Rc<dyn Fn>` that can't be compared structurally. Hand-write it: memoize on the children and the
// handler's identity (pointer equality).
impl PartialEq for ErrorBoundaryProps {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children && Rc::ptr_eq(&self.handle_error.0, &other.handle_error.0)
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
    let error_boundary = use_error_boundary_provider();
    let errors = error_boundary.error();
    let has_errors = errors.is_some();

    // Drop errors before running user code that might borrow the error lock
    drop(errors);

    if has_errors {
        (props.handle_error.0)(error_boundary.clone())
    } else {
        std::result::Result::Ok({
            static STORAGE: TemplateStorage<1, 1, 1> =
                TemplateStorage::build_from_tree(&ERROR_DYNAMIC_TREE);
            static TEMPLATE: Template = STORAGE.as_template();

            VNode::new(
                TEMPLATE,
                DynamicValues::from_parts(
                    None,
                    Box::new([(props.children).into_dyn_node()]),
                    Box::new([]),
                ),
            )
        })
    }
}
