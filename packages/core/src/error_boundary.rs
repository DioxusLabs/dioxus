use crate::{ScopeId, ScopeState};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
};

/// A boundary that will capture any errors from child components
pub struct ErrorBoundary {
    error: RefCell<Option<CapturedError>>,
    _id: ScopeId,
}

/// An instance of an error captured by a descendant component.
pub struct CapturedError {
    /// The error captured by the error boundary
    pub error: Box<dyn Debug + 'static>,

    /// The scope that threw the error
    pub scope: ScopeId,
}

impl CapturedError {
    /// Downcast the error type into a concrete error type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        if TypeId::of::<T>() == self.error.type_id() {
            let raw = self.error.as_ref() as *const _ as *const T;
            Some(unsafe { &*raw })
        } else {
            None
        }
    }
}

impl ErrorBoundary {
    pub fn new(id: ScopeId) -> Self {
        Self {
            error: RefCell::new(None),
            _id: id,
        }
    }

    /// Push an error into this Error Boundary
    pub fn insert_error(&self, scope: ScopeId, error: Box<dyn Debug + 'static>) {
        self.error.replace(Some(CapturedError { error, scope }));
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
/// fn App(cx: Scope, count: String) -> Element {
///     let id: i32 = count.parse().throw(cx)?;
///
///     cx.render(rsx! {
///         div { "Count {}" }
///     })
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
    /// fn App(cx: Scope, count: String) -> Element {
    ///     let id: i32 = count.parse().throw(cx)?;
    ///
    ///     cx.render(rsx! {
    ///         div { "Count {}" }
    ///     })
    /// }
    /// ```
    fn throw(self, cx: &ScopeState) -> Option<Self::Out>;

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
    /// fn App(cx: Scope, count: String) -> Element {
    ///     let id: i32 = count.parse().throw(cx)?;
    ///
    ///     cx.render(rsx! {
    ///         div { "Count {}" }
    ///     })
    /// }
    /// ```
    fn throw_with<D: Debug + 'static>(
        self,
        cx: &ScopeState,
        e: impl FnOnce() -> D,
    ) -> Option<Self::Out>;
}

/// We call clone on any errors that can be owned out of a reference
impl<'a, T, O: Debug + 'static, E: ToOwned<Owned = O>> Throw for &'a Result<T, E> {
    type Out = &'a T;

    fn throw(self, cx: &ScopeState) -> Option<Self::Out> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                cx.throw(e.to_owned());
                None
            }
        }
    }

    fn throw_with<D: Debug + 'static>(
        self,
        cx: &ScopeState,
        err: impl FnOnce() -> D,
    ) -> Option<Self::Out> {
        match self {
            Ok(t) => Some(t),
            Err(_e) => {
                cx.throw(err());
                None
            }
        }
    }
}

/// Or just throw errors we know about
impl<T, E: Debug + 'static> Throw for Result<T, E> {
    type Out = T;

    fn throw(self, cx: &ScopeState) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                cx.throw(e);
                None
            }
        }
    }

    fn throw_with<D: Debug + 'static>(
        self,
        cx: &ScopeState,
        error: impl FnOnce() -> D,
    ) -> Option<Self::Out> {
        self.ok().or_else(|| {
            cx.throw(error());
            None
        })
    }
}

/// Or just throw errors we know about
impl<T> Throw for Option<T> {
    type Out = T;

    fn throw(self, cx: &ScopeState) -> Option<T> {
        self.or_else(|| {
            cx.throw("None error.");
            None
        })
    }

    fn throw_with<D: Debug + 'static>(
        self,
        cx: &ScopeState,
        error: impl FnOnce() -> D,
    ) -> Option<Self::Out> {
        self.or_else(|| {
            cx.throw(error());
            None
        })
    }
}
