//! Typed events for listeners attached to the window or the document.

use super::PlatformEventData;
use dioxus_vdom::{Event, SpawnIfAsync};
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

/// The kinds of object an event can be listened for on.
///
/// The event table lists these in `#[targets(...)]`. Renderers pass the kind an event was
/// listened for on when looking an event up by name, since a name such as `resize` can
/// have a different payload per target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventTargetKind {
    /// An element, as for `rsx!` event handlers.
    Element,
    /// The document.
    Document,
    /// The window.
    Window,
}

#[allow(non_upper_case_globals)]
impl EventTargetKind {
    #[doc(hidden)]
    pub const element: Self = Self::Element;
    #[doc(hidden)]
    pub const document: Self = Self::Document;
    #[doc(hidden)]
    pub const window: Self = Self::Window;
}

/// A target that `use_window_listener!` and `use_document_listener!` can listen on.
pub trait GlobalEventTarget: Sized + 'static {
    #[doc(hidden)]
    const KIND: EventTargetKind;
}

/// The window that owns the current component.
pub struct WindowTarget;

impl GlobalEventTarget for WindowTarget {
    const KIND: EventTargetKind = EventTargetKind::Window;
}

/// The document that owns the current component.
pub struct DocumentTarget;

impl GlobalEventTarget for DocumentTarget {
    const KIND: EventTargetKind = EventTargetKind::Document;
}

/// The layout viewport dimensions in CSS pixels for a window `resize` event.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowResizeData {
    width: f64,
    height: f64,
}

impl WindowResizeData {
    /// Create resize data from the viewport dimensions in CSS pixels.
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// The viewport width in CSS pixels.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// The viewport height in CSS pixels.
    pub fn height(&self) -> f64 {
        self.height
    }
}

/// An event delivered by a renderer to a window or document listener.
pub struct GlobalEventInput {
    /// The renderer's event data: a [`PlatformEventData`], as for element events.
    pub payload: Rc<dyn Any>,
    /// Whether the event bubbles.
    pub bubbles: bool,
}

/// What a window or document listener asked the renderer to do with an event.
#[derive(Default)]
pub struct GlobalEventResult {
    /// The listener called `prevent_default`. The renderer should cancel the event's
    /// default action if the event is cancelable and the listener is not passive.
    pub prevent_default: bool,
}

/// The callback a renderer invokes, synchronously, each time a registered window or
/// document event fires.
pub type GlobalEventCallback = Rc<dyn Fn(GlobalEventInput) -> GlobalEventResult>;

/// An event name paired with its payload type and the target it is listened for on.
///
/// Values come from the methods of [`GlobalEventsExtension`](super::GlobalEventsExtension)
/// and its per-target subtraits; the listener macros attach a handler to them.
pub struct GlobalEvent<Target, Data> {
    name: &'static str,
    marker: PhantomData<fn() -> (Target, Data)>,
}

impl<Target, Data> GlobalEvent<Target, Data>
where
    Target: GlobalEventTarget,
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    #[doc(hidden)]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    /// Attach a handler that receives the event.
    #[doc(hidden)]
    pub fn bind<R, Marker>(
        self,
        handler: impl FnMut(Event<Data>) -> R + 'static,
    ) -> GlobalEventBinding<Target>
    where
        R: SpawnIfAsync<Marker>,
    {
        let handler = RefCell::new(handler);
        let callback = Rc::new(move |input: GlobalEventInput| {
            let Ok(mut handler) = handler.try_borrow_mut() else {
                return GlobalEventResult::default();
            };
            let Some(payload) = input.payload.downcast_ref::<PlatformEventData>() else {
                return GlobalEventResult::default();
            };
            let event = Event::new(Rc::new(Data::from(payload)), input.bubbles);
            handler(event.clone()).spawn();
            GlobalEventResult {
                prevent_default: !event.default_action_enabled(),
            }
        });
        GlobalEventBinding {
            name: self.name,
            callback,
            marker: PhantomData,
        }
    }
}

/// A handler bound to an event on one target, ready to be registered by a listener hook.
#[doc(hidden)]
pub struct GlobalEventBinding<Target> {
    pub name: &'static str,
    pub callback: GlobalEventCallback,
    marker: PhantomData<fn() -> Target>,
}

impl<Target> GlobalEventBinding<Target> {
    #[doc(hidden)]
    pub fn new(name: &'static str, callback: GlobalEventCallback) -> Self {
        Self {
            name,
            callback,
            marker: PhantomData,
        }
    }
}

/// How a renderer should attach a window or document listener.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalListenerOptions {
    /// Run during capture rather than bubble.
    pub capture: bool,
    /// Disallow cancelling the browser's default action.
    pub passive: bool,
}

/// A renderer's support for listeners on the window and the document.
///
/// Renderers provide an `Rc<dyn GlobalListeners>` as a root context. Where none is
/// provided, `use_window_listener!` and `use_document_listener!` in `dioxus-document` install nothing.
///
/// The callback must be called synchronously while the event is being dispatched, and
/// the default action updated from its result before dispatch continues.
pub trait GlobalListeners: 'static {
    /// Attach a listener to the window that owns the current component.
    fn add_window_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration>;

    /// Attach a listener to the document that owns the current component.
    fn add_document_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration>;
}

/// A renderer registration that detaches when dropped.
pub struct GlobalListenerRegistration(Option<Box<dyn FnOnce()>>);

impl GlobalListenerRegistration {
    /// Create a registration that runs `detach` when it is dropped.
    pub fn new(detach: impl FnOnce() + 'static) -> Self {
        Self(Some(Box::new(detach)))
    }
}

impl Drop for GlobalListenerRegistration {
    fn drop(&mut self) {
        if let Some(detach) = self.0.take() {
            detach();
        }
    }
}
