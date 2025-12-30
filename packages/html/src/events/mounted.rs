//! Handles querying data from the renderer

use std::{
    fmt::{Debug, Display, Formatter},
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
        _options: ScrollToOptions,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    /// Scroll to the given element offsets
    fn scroll(
        &self,
        _coordinates: PixelsVector2D,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "ScrollIntoViewOptions")]
pub enum ScrollBehavior {
    /// Scroll to the element immediately
    #[cfg_attr(feature = "serialize", serde(rename = "instant"))]
    Instant,

    /// Scroll to the element smoothly
    #[default]
    #[cfg_attr(feature = "serialize", serde(rename = "smooth"))]
    Smooth,
}

/// The desired final position within the scrollable ancestor container for a given axis.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "ScrollIntoViewOptions")]
pub enum ScrollLogicalPosition {
    /// Aligns the element's start edge (top or left) with the start of the scrollable container,
    /// making the element appear at the start of the visible area.
    #[cfg_attr(feature = "serialize", serde(rename = "start"))]
    Start,
    /// Aligns the element at the center of the scrollable container,
    /// positioning it in the middle of the visible area.
    #[cfg_attr(feature = "serialize", serde(rename = "center"))]
    Center,
    /// Aligns the element's end edge (bottom or right) with the end of the scrollable container,
    /// making the element appear at the end of the visible area
    #[cfg_attr(feature = "serialize", serde(rename = "end"))]
    End,
    /// Scrolls the element to the nearest edge in the given axis.
    /// This minimizes the scrolling distance.
    #[cfg_attr(feature = "serialize", serde(rename = "nearest"))]
    Nearest,
}

/// The way that scrolling should be performed
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "ScrollIntoViewOptions")]
pub struct ScrollToOptions {
    pub behavior: ScrollBehavior,
    pub vertical: ScrollLogicalPosition,
    pub horizontal: ScrollLogicalPosition,
}
impl Default for ScrollToOptions {
    fn default() -> Self {
        Self {
            behavior: ScrollBehavior::Smooth,
            vertical: ScrollLogicalPosition::Start,
            horizontal: ScrollLogicalPosition::Center,
        }
    }
}

/// An Element that has been rendered and allows reading and modifying information about it.
///
/// Different platforms will have different implementations and different levels of support for this trait. Renderers that do not support specific features will return `None` for those queries.
pub struct MountedData {
    inner: Box<dyn RenderedElementBacking>,
    /// Cleanup closure to run when the element is unmounted.
    /// Stored via interior mutability so handlers can set it on shared references.
    cleanup: RefCell<Option<Box<dyn FnOnce() + 'static>>>,
}

impl Debug for MountedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MountedData").finish()
    }
}

impl<E: RenderedElementBacking> From<E> for MountedData {
    fn from(e: E) -> Self {
        Self {
            inner: Box::new(e),
            cleanup: RefCell::new(None),
        }
    }
}

impl MountedData {
    /// Create a new MountedData
    pub fn new(registry: impl RenderedElementBacking + 'static) -> Self {
        Self {
            inner: Box::new(registry),
            cleanup: RefCell::new(None),
        }
    }

    /// Store a cleanup closure to run when the element is unmounted.
    ///
    /// This is called by the handler via `event.data().set_on_cleanup(closure)`.
    /// The renderer will retrieve and invoke this cleanup when the element is freed.
    pub fn set_on_cleanup(&self, cleanup: impl FnOnce() + 'static) {
        *self.cleanup.borrow_mut() = Some(Box::new(cleanup));
    }

    /// Take the cleanup closure, if any was registered.
    ///
    /// Called by renderers after the mounted event handler returns to retrieve
    /// any cleanup closure that should be invoked when the element is unmounted.
    pub fn take_cleanup(&self) -> Option<Box<dyn FnOnce() + 'static>> {
        self.cleanup.borrow_mut().take()
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
        self.inner.scroll_to(ScrollToOptions {
            behavior,
            ..ScrollToOptions::default()
        })
    }

    /// Scroll to make the element visible
    #[doc(alias = "scrollIntoView")]
    pub fn scroll_to_with_options(
        &self,
        options: ScrollToOptions,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.scroll_to(options)
    }

    /// Scroll to the given element offsets
    #[doc(alias = "scrollTo")]
    pub fn scroll(
        &self,
        coordinates: PixelsVector2D,
        behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.scroll(coordinates, behavior)
    }

    /// Set the focus on the element
    #[doc(alias = "focus")]
    #[doc(alias = "blur")]
    pub fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        self.inner.set_focus(focus)
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

use std::cell::RefCell;

use dioxus_core::Event;

use crate::geometry::{PixelsRect, PixelsSize, PixelsVector2D};
use crate::PlatformEventData;

pub type MountedEvent = Event<MountedData>;

// ============================================================================
// Cleanup support for onmounted
// ============================================================================

/// Trait to allow onmounted handlers to optionally return a cleanup closure.
///
/// This enables the pattern:
/// ```rust,ignore
/// onmounted: move |e| {
///     start_animation(e.data());
///     move || stop_animation(e.data())  // cleanup returned
/// }
/// ```
///
/// Handlers can return:
/// - `()` - no cleanup
/// - Any `FnOnce()` closure - cleanup to run on unmount
/// - `async {}` block - spawned as task, no cleanup support
pub trait SpawnIfAsyncWithCleanup<Marker>: Sized {
    /// Process the return value, storing any cleanup closure on the MountedData.
    fn spawn_with_cleanup(self, data: &MountedData);
}

// No cleanup - handler returns ()
impl SpawnIfAsyncWithCleanup<()> for () {
    fn spawn_with_cleanup(self, _data: &MountedData) {
        // No cleanup needed
    }
}

/// Marker for cleanup closures
#[doc(hidden)]
pub struct CleanupMarker;

// Handler returns a cleanup closure
impl<F: FnOnce() + 'static> SpawnIfAsyncWithCleanup<CleanupMarker> for F {
    fn spawn_with_cleanup(self, data: &MountedData) {
        data.set_on_cleanup(self);
    }
}

/// Marker for async handlers (no cleanup support for async yet)
#[doc(hidden)]
pub struct AsyncMountedMarker;

impl<F: std::future::Future<Output = ()> + 'static> SpawnIfAsyncWithCleanup<AsyncMountedMarker>
    for F
{
    fn spawn_with_cleanup(self, _data: &MountedData) {
        // Spawn the async block but no cleanup support
        use futures_util::FutureExt;
        let mut fut = Box::pin(self);
        let res = fut.as_mut().now_or_never();

        if res.is_none() {
            dioxus_core::spawn(async move {
                fut.await;
            });
        }
    }
}

// ============================================================================
// onmounted event handler
// ============================================================================

#[doc(alias = "ref")]
#[doc(alias = "createRef")]
#[doc(alias = "useRef")]
/// The onmounted event is fired when the element is first added to the DOM. This event gives you a [`MountedData`] object and lets you interact with the raw DOM element.
///
/// This event is fired once per element. If you need to access the element multiple times, you can store the [`MountedData`] object in a [`use_signal`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_signal.html) hook and use it as needed.
///
/// You can optionally return a cleanup closure that will be called when the element is removed from the DOM:
///
/// # Examples
///
/// ## Basic usage (no cleanup)
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
/// ## With cleanup closure
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn App() -> Element {
///     let mut cleanup_called = use_signal(|| false);
///
///     rsx! {
///         div {
///             onmounted: move |_| {
///                 // Return a cleanup closure that runs when the element is removed
///                 move || {
///                     cleanup_called.set(true);
///                 }
///             },
///             "Element with cleanup"
///         }
///     }
/// }
/// ```
///
/// The `MountedData` struct contains cross platform APIs that work on the desktop, mobile, liveview and web platforms. For the web platform, you can also downcast the `MountedData` event to the `web-sys::Element` type for more web specific APIs:
///
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use dioxus_web::WebEventExt; // provides [`as_web_event()`] method
///
/// fn App() -> Element {
///     rsx! {
///         div {
///             id: "some-id",
///             onmounted: move |element| {
///                 // You can use the web_event trait to downcast the element to a web specific event. For the mounted event, this will be a web_sys::Element
///                 let web_sys_element = element.as_web_event();
///                 assert_eq!(web_sys_element.id(), "some-id");
///             }
///         }
///     }
/// }
/// ```
#[inline]
pub fn onmounted<__Marker>(
    f: impl ::dioxus_core::SuperInto<::dioxus_core::ListenerCallback<MountedData>, __Marker>,
) -> ::dioxus_core::Attribute {
    let event_handler = f.super_into();
    // Use new_raw to handle both MountedData (from renderer) and PlatformEventData (legacy)
    let listener = ::dioxus_core::ListenerCallback::<MountedData>::new_raw(
        move |e: ::dioxus_core::Event<dyn std::any::Any>| {
            // Try to get MountedData directly (renderer-created for cleanup support)
            // Otherwise fall back to PlatformEventData for backwards compatibility
            let mounted_data: std::rc::Rc<MountedData> =
                if let Ok(data) = e.data.clone().downcast::<MountedData>() {
                    // Renderer passed MountedData directly - use the same Rc for cleanup retrieval
                    data
                } else if let Ok(platform_data) = e.data.clone().downcast::<PlatformEventData>() {
                    // Legacy path: convert PlatformEventData to MountedData
                    // Note: cleanup callbacks won't work with this path - renderers should send Rc<MountedData> directly
                    std::rc::Rc::new((&*platform_data).into())
                } else {
                    // Unexpected type - log error and return
                    tracing::error!("onmounted received unexpected event data type");
                    return;
                };

            // Use with_data to create event with same metadata but our MountedData
            let event = e.with_data(mounted_data);
            event_handler.call(event.into_any());
        },
    );
    ::dioxus_core::Attribute::new(
        "onmounted", // Core strips "on" prefix when matching
        ::dioxus_core::AttributeValue::Listener(listener.erase()),
        None,
        false,
    )
}

#[doc(hidden)]
pub mod onmounted {
    use super::*;

    /// Called by RSX macro when explicit closure is detected.
    /// Uses SpawnIfAsyncWithCleanup to handle cleanup return values.
    pub fn call_with_explicit_closure<
        __Marker,
        Return: SpawnIfAsyncWithCleanup<__Marker> + 'static,
    >(
        mut handler: impl FnMut(::dioxus_core::Event<MountedData>) -> Return + 'static,
    ) -> ::dioxus_core::Attribute {
        // Wrap the handler to process the return value for cleanup
        super::onmounted(move |event: ::dioxus_core::Event<MountedData>| {
            let result = handler(event.clone());
            // Process the result - this handles (), FnOnce cleanup closures, and async futures
            // Store cleanup on the shared MountedData so renderers can retrieve it
            result.spawn_with_cleanup(&event.data);
        })
    }
}

pub use onmounted as onmount;

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
