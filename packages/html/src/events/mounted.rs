//! Handles quering data from the renderer

use euclid::Rect;

use std::{any::Any, rc::Rc};

/// An Element that has been rendered and allows reading and modifying information about it.
///
/// Different platforms will have different implementations and different levels of support for this trait. Renderers that do not support specific features will return `None` for those queries.
pub trait RenderedElementBacking {
    /// Get the renderer specific element for the given id
    fn get_raw_element(&self) -> Option<&dyn Any> {
        None
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    fn get_client_rect(&self) -> Option<Rect<f64, f64>> {
        None
    }

    /// Scroll to make the element visible
    fn scroll_to(&self, _behavior: ScrollBehavior) -> Option<()> {
        None
    }

    /// Set the focus on the element
    fn set_focus(&self, _focus: bool) -> Option<()> {
        None
    }
}

/// The way that scrolling should be performed
pub enum ScrollBehavior {
    /// Scroll to the element immediately
    Instant,
    /// Scroll to the element smoothly
    Smooth,
}

/// An Element that has been rendered and allows reading and modifying information about it.
///
/// Different platforms will have different implementations and different levels of support for this trait. Renderers that do not support specific features will return `None` for those queries.
pub struct MountedData {
    inner: Rc<dyn RenderedElementBacking>,
}

impl MountedData {
    /// Create a new MountedData
    pub fn new(registry: impl RenderedElementBacking + 'static) -> Self {
        Self {
            inner: Rc::new(registry),
        }
    }

    /// Get the renderer specific element for the given id
    pub fn get_raw_element(&self) -> Option<&dyn Any> {
        self.inner.get_raw_element()
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    pub fn get_client_rect(&self) -> Option<Rect<f64, f64>> {
        self.inner.get_client_rect()
    }

    /// Scroll to make the element visible
    pub fn scroll_to(&self, behavior: ScrollBehavior) -> Option<()> {
        self.inner.scroll_to(behavior)
    }

    /// Set the focus on the element
    pub fn set_focus(&self, focus: bool) -> Option<()> {
        self.inner.set_focus(focus)
    }
}

use dioxus_core::Event;

pub type MountedEvent = Event<MountedData>;

impl_event! [
    MountedData;

    /// mounted
    onmounted
];
