//! Typed events for listeners attached to the window or the document.

use super::PlatformEventData;
use dioxus_vdom::{Event, SpawnIfAsync};
use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

/// The kinds of object an event can be listened for on.
///
/// The event table lists these in `#[targets(...)]`.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventTargetKind {
    Element,
    Document,
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
#[doc(hidden)]
pub struct GlobalEventInput {
    /// The renderer's event data: a [`PlatformEventData`], as for element events.
    pub payload: Rc<dyn Any>,
    pub bubbles: bool,
    pub cancelable: bool,
    pub default_prevented: bool,
}

/// What a window or document listener asked the renderer to do with an event.
#[doc(hidden)]
#[derive(Default)]
pub struct GlobalEventResult {
    pub prevent_default: bool,
    pub stop_propagation: bool,
}

#[doc(hidden)]
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
            let event = Event::new_with_flags(
                Rc::new(Data::from(payload)),
                input.bubbles,
                input.cancelable,
                input.default_prevented,
            );
            handler(event.clone()).spawn();
            GlobalEventResult {
                prevent_default: !input.default_prevented && !event.default_action_enabled(),
                stop_propagation: event.propagation_stopped(),
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
