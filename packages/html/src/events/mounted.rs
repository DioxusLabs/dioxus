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
pub trait RenderedElementBacking {
    fn id(&self) -> usize;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = Rect<f64, f64>>>>;

    /// Scroll to make the element visible
    fn scroll_to(&self, _behavior: ScrollBehavior) -> Pin<Box<dyn Future<Output = ()>>>;

    /// Set the focus on the element
    fn set_focus(&self, _focus: bool) -> Pin<Box<dyn Future<Output = ()>>>;
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

impl MountedData {
    /// Create a new MountedData
    pub fn new(registry: impl RenderedElementBacking + 'static) -> Self {
        println!("MountedData::new");
        Self {
            inner: Box::new(registry),
        }
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    pub async fn get_client_rect(&self) -> Rect<f64, f64> {
        self.inner.get_client_rect().await
    }

    /// Scroll to make the element visible
    pub fn scroll_to(&self, behavior: ScrollBehavior) -> Pin<Box<dyn Future<Output = ()>>> {
        self.inner.scroll_to(behavior)
    }

    /// Set the focus on the element
    pub fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = ()>>> {
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
