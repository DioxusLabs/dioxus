//! Handles querying data from the renderer

use euclid::Rect;

use std::{
    fmt::{Display, Formatter},
    future::Future,
    pin::Pin,
};

/// An Element that has been rendered and allows reading and modifying information about it.
///
/// Different platforms will have different implementations and different levels of support for this trait. Renderers that do not support specific features will return `None` for those queries.
// we can not use async_trait here because it does not create a trait that is object safe
pub trait RenderedElementBacking: std::any::Any {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    #[allow(clippy::type_complexity)]
    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<Rect<f64, f64>>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    /// Scroll to make the element visible
    fn scroll_to(
        &self,
        _behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    /// Set the focus on the element
    fn set_focus(&self, _focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }
}

impl RenderedElementBacking for () {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The way that scrolling should be performed
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum ScrollBehavior {
    /// Scroll to the element immediately
    #[cfg_attr(feature = "serialize", serde(rename = "instant"))]
    Instant,
    /// Scroll to the element smoothly
    #[cfg_attr(feature = "serialize", serde(rename = "smooth"))]
    Smooth,
}

/// An Element that has been rendered and allows reading and modifying information about it.
///
/// Different platforms will have different implementations and different levels of support for this trait. Renderers that do not support specific features will return `None` for those queries.
pub struct MountedData {
    inner: Box<dyn RenderedElementBacking>,
}

impl<E: RenderedElementBacking> From<E> for MountedData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl MountedData {
    /// Create a new MountedData
    pub fn new(registry: impl RenderedElementBacking + 'static) -> Self {
        Self {
            inner: Box::new(registry),
        }
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    pub async fn get_client_rect(&self) -> MountedResult<Rect<f64, f64>> {
        self.inner.get_client_rect().await
    }

    /// Scroll to make the element visible
    pub fn scroll_to(
        &self,
        behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.scroll_to(behavior)
    }

    /// Set the focus on the element
    pub fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.set_focus(focus)
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

use dioxus_core::Event;

pub type MountedEvent = Event<MountedData>;

impl_event! [
    MountedData;

    /// mounted
    onmounted
];

/// The MountedResult type for the MountedData
pub type MountedResult<T> = Result<T, MountedError>;

#[derive(Debug)]
/// The error type for the MountedData
pub enum MountedError {
    /// The renderer does not support the requested operation
    NotSupported,
    /// The element was not found
    OperationFailed(Box<dyn std::error::Error>),
}

impl Display for MountedError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MountedError::NotSupported => {
                write!(f, "The renderer does not support the requested operation")
            }
            MountedError::OperationFailed(e) => {
                write!(f, "The operation failed: {}", e)
            }
        }
    }
}

impl std::error::Error for MountedError {}
