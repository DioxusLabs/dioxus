#![doc = include_str!("../../docs/event_handlers.md")]

use std::any::Any;
use std::sync::RwLock;

static EVENT_CONVERTER: RwLock<Option<Box<dyn HtmlEventConverter>>> = RwLock::new(None);

#[inline]
pub fn set_event_converter(converter: Box<dyn HtmlEventConverter>) {
    *EVENT_CONVERTER.write().unwrap() = Some(converter);
}

#[inline]
pub(crate) fn with_event_converter<F, R>(f: F) -> R
where
    F: FnOnce(&dyn HtmlEventConverter) -> R,
{
    let converter = EVENT_CONVERTER.read().unwrap();
    f(converter.as_ref().unwrap().as_ref())
}

/// A platform specific event.
pub struct PlatformEventData {
    event: Box<dyn Any>,
}

impl PlatformEventData {
    pub fn new(event: Box<dyn Any>) -> Self {
        Self { event }
    }

    pub fn inner(&self) -> &Box<dyn Any> {
        &self.event
    }

    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.event.downcast_ref::<T>()
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.event.downcast_mut::<T>()
    }

    pub fn into_inner<T: 'static>(self) -> Option<T> {
        self.event.downcast::<T>().ok().map(|e| *e)
    }
}

/// Marker for event handlers provided as closures.
pub struct EventClosureMarker<Marker>(std::marker::PhantomData<Marker>);

/// Marker for event handlers provided as listener callbacks.
#[doc(hidden)]
pub struct EventListenerMarker;

/// Marker for event handlers provided as callbacks.
#[doc(hidden)]
pub struct EventCallbackMarker;

/// A value that can be converted into a platform event listener.
pub trait EventHandlerValue<Data, Marker>
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    /// Convert this handler into a platform event listener.
    fn into_platform_listener(self) -> ::dioxus_core::ListenerCallback<PlatformEventData>;
}

impl<Data, Function, Spawn, Marker> EventHandlerValue<Data, EventClosureMarker<Marker>> for Function
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
    Function: FnMut(::dioxus_core::Event<Data>) -> Spawn + 'static,
    Spawn: ::dioxus_core::SpawnIfAsync<Marker> + 'static,
{
    fn into_platform_listener(mut self) -> ::dioxus_core::ListenerCallback<PlatformEventData> {
        ::dioxus_core::ListenerCallback::new(
            move |event: ::dioxus_core::Event<PlatformEventData>| {
                self(event.map(|data| Data::from(data)))
            },
        )
    }
}

impl<Data> EventHandlerValue<Data, EventListenerMarker> for ::dioxus_core::ListenerCallback<Data>
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    fn into_platform_listener(self) -> ::dioxus_core::ListenerCallback<PlatformEventData> {
        ::dioxus_core::ListenerCallback::new(
            move |event: ::dioxus_core::Event<PlatformEventData>| {
                self.call(event.map(|data| Data::from(data)).into_any());
            },
        )
    }
}

impl<Data> EventHandlerValue<Data, EventCallbackMarker>
    for ::dioxus_core::Callback<::dioxus_core::Event<Data>>
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    fn into_platform_listener(self) -> ::dioxus_core::ListenerCallback<PlatformEventData> {
        ::dioxus_core::ListenerCallback::new(
            move |event: ::dioxus_core::Event<PlatformEventData>| {
                self.call(event.map(|data| Data::from(data)));
            },
        )
    }
}

pub(crate) fn event_attribute<Data, Marker>(
    name: &'static str,
    event_handler: impl EventHandlerValue<Data, Marker>,
) -> ::dioxus_core::Attribute
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    ::dioxus_core::Attribute::new(
        name,
        ::dioxus_core::AttributeValue::Listener(event_handler.into_platform_listener().erase()),
        None,
        false,
    )
}

mod generated;

mod animation;
mod before_input;
mod cancel;
mod clipboard;
mod composition;
mod drag;
mod focus;
mod form;
mod image;
mod keyboard;
mod media;
mod mounted;
mod mouse;
mod pointer;
mod resize;
mod scroll;
mod selection;
mod toggle;
mod touch;
mod transition;
mod visible;
mod wheel;

pub use animation::*;
pub use before_input::*;
pub use cancel::*;
pub use clipboard::*;
pub use composition::*;
pub use drag::*;
pub use focus::*;
pub use form::*;
pub use generated::*;
pub use image::*;
pub use keyboard::*;
pub use media::*;
pub use mounted::*;
pub use mouse::*;
pub use pointer::*;
pub use resize::*;
pub use scroll::*;
pub use selection::*;
pub use toggle::*;
pub use touch::*;
pub use transition::*;
pub use visible::*;
pub use wheel::*;
