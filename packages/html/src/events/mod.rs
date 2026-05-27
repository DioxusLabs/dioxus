#![doc = include_str!("../../docs/event_handlers.md")]

use std::any::Any;
use std::sync::RwLock;

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident $( : $js_name:expr )?;
        )*
    ) => {
        $(
            $( #[$attr] )*
            /// <details open>
            /// <summary>General Event Handler Information</summary>
            ///
            #[doc = include_str!("../../docs/event_handlers.md")]
            ///
            /// </details>
            ///
            #[doc = include_str!("../../docs/common_event_handler_errors.md")]
            #[inline]
            pub fn $name<__Marker>(mut _f: impl ::dioxus_core::SuperInto<::dioxus_core::ListenerCallback<$data>, __Marker>) -> ::dioxus_core::Attribute {
                let event_handler = _f.super_into();
                ::dioxus_core::Attribute::new(
                    impl_event!(@name $name $($js_name)?),
                    ::dioxus_core::AttributeValue::listener(move |e: ::dioxus_core::Event<crate::PlatformEventData>| {
                        let event: ::dioxus_core::Event<$data> = e.map(|data| {
                            data.into()
                        });
                        event_handler.call(event.into_any());
                    }),
                    None,
                    false,
                ).into()
            }

            #[doc(hidden)]
            $( #[$attr] )*
            pub mod $name {
                use super::*;

                // When expanding the macro, we use this version of the function if we see an inline closure to give better type inference
                $( #[$attr] )*
                pub fn call_with_explicit_closure<
                    __Marker,
                    Return: ::dioxus_core::SpawnIfAsync<__Marker> + 'static,
                >(
                    event_handler: impl FnMut(::dioxus_core::Event<$data>) -> Return + 'static,
                ) -> ::dioxus_core::Attribute {
                    #[allow(deprecated)]
                    super::$name(event_handler)
                }
            }
        )*
    };

    (@name $name:ident $js_name:expr) => {
        $js_name
    };
    (@name $name:ident) => {
        stringify!($name)
    };
}

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

mod generated;

mod animation;
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
