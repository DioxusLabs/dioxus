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

#[doc(hidden)]
pub struct EventClosureMarker<Marker>(std::marker::PhantomData<Marker>);

#[doc(hidden)]
pub struct EventListenerMarker;

#[doc(hidden)]
pub struct EventCallbackMarker;

#[doc(hidden)]
pub trait EventHandlerValue<Data, Marker> {
    fn into_listener(self) -> ::dioxus_core::ListenerCallback<Data>;
}

impl<Data, Function, Spawn, Marker> EventHandlerValue<Data, EventClosureMarker<Marker>> for Function
where
    Data: 'static,
    Function: FnMut(::dioxus_core::Event<Data>) -> Spawn + 'static,
    Spawn: ::dioxus_core::SpawnIfAsync<Marker> + 'static,
{
    fn into_listener(self) -> ::dioxus_core::ListenerCallback<Data> {
        ::dioxus_core::ListenerCallback::new(self)
    }
}

impl<Data> EventHandlerValue<Data, EventListenerMarker> for ::dioxus_core::ListenerCallback<Data> {
    fn into_listener(self) -> ::dioxus_core::ListenerCallback<Data> {
        self
    }
}

impl<Data> EventHandlerValue<Data, EventCallbackMarker>
    for ::dioxus_core::Callback<::dioxus_core::Event<Data>>
where
    Data: 'static,
{
    fn into_listener(self) -> ::dioxus_core::ListenerCallback<Data> {
        ::dioxus_core::ListenerCallback::new(move |event| self.call(event))
    }
}

pub(crate) fn event_attribute<Data, Marker>(
    name: &'static str,
    event_handler: impl EventHandlerValue<Data, Marker>,
) -> ::dioxus_core::Attribute
where
    Data: for<'a> From<&'a PlatformEventData> + 'static,
{
    let event_handler = event_handler.into_listener();
    ::dioxus_core::Attribute::new(
        name,
        ::dioxus_core::AttributeValue::listener(
            move |event: ::dioxus_core::Event<PlatformEventData>| {
                let event = event.map(|data| Data::from(data));
                event_handler.call(event.into_any());
            },
        ),
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
