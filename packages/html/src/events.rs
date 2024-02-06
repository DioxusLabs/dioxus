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
            #[inline]
            pub fn $name<E: crate::EventReturn<T>, T>(mut _f: impl FnMut(::dioxus_core::Event<$data>) -> E + 'static) -> ::dioxus_core::Attribute {
                ::dioxus_core::Attribute::new(
                    impl_event!(@name $name $($js_name)?),
::dioxus_core::AttributeValue::listener(move |e: ::dioxus_core::Event<crate::PlatformEventData>| {
                        _f(e.map(|e|e.into())).spawn();
                    }),
                    None,
                    false,
                ).into()
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
pub use scroll::*;
pub use selection::*;
pub use toggle::*;
pub use touch::*;
pub use transition::*;
pub use wheel::*;

pub fn event_bubbles(evt: &str) -> bool {
    match evt {
        "copy" => true,
        "cut" => true,
        "paste" => true,
        "compositionend" => true,
        "compositionstart" => true,
        "compositionupdate" => true,
        "keydown" => true,
        "keypress" => true,
        "keyup" => true,
        "focus" => false,
        "focusout" => true,
        "focusin" => true,
        "blur" => false,
        "change" => true,
        "input" => true,
        "invalid" => true,
        "reset" => true,
        "submit" => true,
        "click" => true,
        "contextmenu" => true,
        "doubleclick" => true,
        "dblclick" => true,
        "drag" => true,
        "dragend" => true,
        "dragenter" => false,
        "dragexit" => false,
        "dragleave" => true,
        "dragover" => true,
        "dragstart" => true,
        "drop" => true,
        "mousedown" => true,
        "mouseenter" => false,
        "mouseleave" => false,
        "mousemove" => true,
        "mouseout" => true,
        "scroll" => false,
        "mouseover" => true,
        "mouseup" => true,
        "pointerdown" => true,
        "pointermove" => true,
        "pointerup" => true,
        "pointercancel" => true,
        "gotpointercapture" => true,
        "lostpointercapture" => true,
        "pointerenter" => false,
        "pointerleave" => false,
        "pointerover" => true,
        "pointerout" => true,
        "select" => true,
        "touchcancel" => true,
        "touchend" => true,
        "touchmove" => true,
        "touchstart" => true,
        "wheel" => true,
        "abort" => false,
        "canplay" => false,
        "canplaythrough" => false,
        "durationchange" => false,
        "emptied" => false,
        "encrypted" => true,
        "ended" => false,
        "error" => false,
        "loadeddata" => false,
        "loadedmetadata" => false,
        "loadstart" => false,
        "load" => false,
        "pause" => false,
        "play" => false,
        "playing" => false,
        "progress" => false,
        "ratechange" => false,
        "seeked" => false,
        "seeking" => false,
        "stalled" => false,
        "suspend" => false,
        "timeupdate" => false,
        "volumechange" => false,
        "waiting" => false,
        "animationstart" => true,
        "animationend" => true,
        "animationiteration" => true,
        "transitionend" => true,
        "toggle" => true,
        "mounted" => false,
        _ => true,
    }
}

#[doc(hidden)]
pub trait EventReturn<P>: Sized {
    fn spawn(self) {}
}

impl EventReturn<()> for () {}
#[doc(hidden)]
pub struct AsyncMarker;

impl<T> EventReturn<AsyncMarker> for T
where
    T: std::future::Future<Output = ()> + 'static,
{
    #[inline]
    fn spawn(self) {
        dioxus_core::prelude::spawn(self);
    }
}
