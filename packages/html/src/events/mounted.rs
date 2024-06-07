//! Handles querying data from the renderer

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

    /// Get the number of pixels that an element's content is scrolled
    fn get_scroll_offset(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsVector2D>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    /// Get the size of an element's content, including content not visible on the screen due to overflow
    #[allow(clippy::type_complexity)]
    fn get_scroll_size(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsSize>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    #[allow(clippy::type_complexity)]
    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
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
#[doc(alias = "ScrollIntoViewOptions")]
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

    /// Get the number of pixels that an element's content is scrolled
    #[doc(alias = "scrollTop")]
    #[doc(alias = "scrollLeft")]
    pub async fn get_scroll_offset(&self) -> MountedResult<PixelsVector2D> {
        self.inner.get_scroll_offset().await
    }

    /// Get the size of an element's content, including content not visible on the screen due to overflow
    #[doc(alias = "scrollWidth")]
    #[doc(alias = "scrollHeight")]
    pub async fn get_scroll_size(&self) -> MountedResult<PixelsSize> {
        self.inner.get_scroll_size().await
    }

    /// Get the bounding rectangle of the element relative to the viewport (this does not include the scroll position)
    #[doc(alias = "getBoundingClientRect")]
    pub async fn get_client_rect(&self) -> MountedResult<PixelsRect> {
        self.inner.get_client_rect().await
    }

    /// Scroll to make the element visible
    #[doc(alias = "scrollIntoView")]
    pub fn scroll_to(
        &self,
        behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.scroll_to(behavior)
    }

    /// Set the focus on the element
    #[doc(alias = "focus")]
    #[doc(alias = "blur")]
    pub fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.set_focus(focus)
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

use dioxus_core::Event;

use crate::geometry::{PixelsRect, PixelsSize, PixelsVector2D};

pub type MountedEvent = Event<MountedData>;

impl_event! [
    MountedData;

    #[doc(alias = "ref")]
    #[doc(alias = "createRef")]
    #[doc(alias = "useRef")]
    /// The onmounted event is fired when the element is first added to the DOM. This event gives you a [`MountedData`] object and lets you interact with the raw DOM element.
    ///
    /// This event is fired once per element. If you need to access the element multiple times, you can store the [`MountedData`] object in a [`use_signal`] hook and use it as needed.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// fn App() -> Element {
    ///     let mut header_element = use_signal(|| None);
    ///
    ///     rsx! {
    ///         div {
    ///             h1 {
    ///                 // The onmounted event will run the first time the h1 element is mounted
    ///                 onmounted: move |element| header_element.set(Some(element.data())),
    ///                 "Scroll to top example"
    ///             }
    ///
    ///             for i in 0..100 {
    ///                 div { "Item {i}" }
    ///             }
    ///
    ///             button {
    ///                 // When you click the button, if the header element has been mounted, we scroll to that element
    ///                 onclick: move |_| async move {
    ///                     if let Some(header) = header_element.cloned() {
    ///                         let _ = header.scroll_to(ScrollBehavior::Smooth).await;
    ///                     }
    ///                 },
    ///                 "Scroll to top"
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// The `MountedData` struct contains cross platform APIs that work on the desktop, mobile, liveview and web platforms. For the web platform, you can also downcast the `MountedData` event to the `web-sys::Element` type for more web specific APIs:
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # use dioxus_web::WebEventExt;
    /// fn App() -> Element {
    ///     rsx! {
    ///         div {
    ///             id: "some-id",
    ///             onmounted: move |element| {
    ///                 // You can use the web_event trait to downcast the element to a web specific event. For the mounted event, this will be a web_sys::Element
    ///                 let web_sys_element = element.web_event();
    ///                 assert_eq!(web_sys_element.id(), "some-id");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    onmounted
];

/// The MountedResult type for the MountedData
pub type MountedResult<T> = Result<T, MountedError>;

#[derive(Debug)]
/// The error type for the MountedData
#[non_exhaustive]
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
