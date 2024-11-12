#![doc = include_str!("../../docs/event_handlers.md")]

use std::any::Any;
use std::sync::RwLock;

macro_rules! impl_event {
    (
        $data:ty;
        $(
            $( #[$attr:meta] )*
            $name:ident $(: $js_name:literal)?
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
            $(
                #[doc(alias = $js_name)]
            )?
            #[inline]
            pub fn $name<__Marker>(mut _f: impl ::dioxus_core::prelude::SuperInto<::dioxus_core::prelude::EventHandler<::dioxus_core::Event<$data>>, __Marker>) -> ::dioxus_core::Attribute {
                let event_handler = _f.super_into();
                ::dioxus_core::Attribute::new(
                    impl_event!(@name $name $($js_name)?),
                    ::dioxus_core::AttributeValue::listener(move |e: ::dioxus_core::Event<crate::PlatformEventData>| {
                        event_handler.call(e.map(|e| e.into()));
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

    (@name $name:ident $js_name:literal) => {
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

/// A converter between a platform specific event and a general event. All code in a renderer that has a large binary size should be placed in this trait. Each of these functions should be snipped in high levels of optimization.
pub trait HtmlEventConverter: Send + Sync {
    /// Convert a general event to an animation data event
    fn convert_animation_data(&self, event: &PlatformEventData) -> AnimationData;
    /// Convert a general event to a clipboard data event
    fn convert_clipboard_data(&self, event: &PlatformEventData) -> ClipboardData;
    /// Convert a general event to a composition data event
    fn convert_composition_data(&self, event: &PlatformEventData) -> CompositionData;
    /// Convert a general event to a drag data event
    fn convert_drag_data(&self, event: &PlatformEventData) -> DragData;
    /// Convert a general event to a focus data event
    fn convert_focus_data(&self, event: &PlatformEventData) -> FocusData;
    /// Convert a general event to a form data event
    fn convert_form_data(&self, event: &PlatformEventData) -> FormData;
    /// Convert a general event to an image data event
    fn convert_image_data(&self, event: &PlatformEventData) -> ImageData;
    /// Convert a general event to a keyboard data event
    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData;
    /// Convert a general event to a media data event
    fn convert_media_data(&self, event: &PlatformEventData) -> MediaData;
    /// Convert a general event to a mounted data event
    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData;
    /// Convert a general event to a mouse data event
    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData;
    /// Convert a general event to a pointer data event
    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData;
    /// Convert a general event to a resize data event
    fn convert_resize_data(&self, event: &PlatformEventData) -> ResizeData;
    /// Convert a general event to a scroll data event
    fn convert_scroll_data(&self, event: &PlatformEventData) -> ScrollData;
    /// Convert a general event to a selection data event
    fn convert_selection_data(&self, event: &PlatformEventData) -> SelectionData;
    /// Convert a general event to a toggle data event
    fn convert_toggle_data(&self, event: &PlatformEventData) -> ToggleData;
    /// Convert a general event to a touch data event
    fn convert_touch_data(&self, event: &PlatformEventData) -> TouchData;
    /// Convert a general event to a transition data event
    fn convert_transition_data(&self, event: &PlatformEventData) -> TransitionData;
    /// Convert a general event to a wheel data event
    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData;
}

impl From<&PlatformEventData> for AnimationData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_animation_data(val))
    }
}

impl From<&PlatformEventData> for ClipboardData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_clipboard_data(val))
    }
}

impl From<&PlatformEventData> for CompositionData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_composition_data(val))
    }
}

impl From<&PlatformEventData> for DragData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_drag_data(val))
    }
}

impl From<&PlatformEventData> for FocusData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_focus_data(val))
    }
}

impl From<&PlatformEventData> for FormData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_form_data(val))
    }
}

impl From<&PlatformEventData> for ImageData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_image_data(val))
    }
}

impl From<&PlatformEventData> for KeyboardData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_keyboard_data(val))
    }
}

impl From<&PlatformEventData> for MediaData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_media_data(val))
    }
}

impl From<&PlatformEventData> for MountedData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_mounted_data(val))
    }
}

impl From<&PlatformEventData> for MouseData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_mouse_data(val))
    }
}

impl From<&PlatformEventData> for PointerData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_pointer_data(val))
    }
}

impl From<&PlatformEventData> for ResizeData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_resize_data(val))
    }
}

impl From<&PlatformEventData> for ScrollData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_scroll_data(val))
    }
}

impl From<&PlatformEventData> for SelectionData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_selection_data(val))
    }
}

impl From<&PlatformEventData> for ToggleData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_toggle_data(val))
    }
}

impl From<&PlatformEventData> for TouchData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_touch_data(val))
    }
}

impl From<&PlatformEventData> for TransitionData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_transition_data(val))
    }
}

impl From<&PlatformEventData> for WheelData {
    fn from(val: &PlatformEventData) -> Self {
        with_event_converter(|c| c.convert_wheel_data(val))
    }
}

mod animation;
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
mod wheel;

pub use animation::*;
pub use clipboard::*;
pub use composition::*;
pub use drag::*;
pub use focus::*;
pub use form::*;
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
pub use wheel::*;
