use dioxus_html::{
    DragData, FormData, HtmlEventConverter, ImageData, MountedData, PlatformEventData,
};
use form::WebFormData;
use load::WebImageEvent;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

mod animation;
mod cancel;
mod clipboard;
mod composition;
mod drag;
mod file;
mod focus;
mod form;
mod keyboard;
mod load;
mod media;
#[cfg(feature = "mounted")]
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

/// A wrapper for the websys event that allows us to give it the impls from dioxus-html
pub(crate) struct Synthetic<T: 'static> {
    /// The inner web sys event that the synthetic event wraps
    pub event: T,
}

impl<T: 'static> Synthetic<T> {
    /// Create a new synthetic event from a web sys event
    pub fn new(event: T) -> Self {
        Self { event }
    }
}

pub(crate) struct WebEventConverter;

#[inline(always)]
fn downcast_event(event: &dioxus_html::PlatformEventData) -> &GenericWebSysEvent {
    event
        .downcast::<GenericWebSysEvent>()
        .expect("event should be a GenericWebSysEvent")
}

/// Single source of truth for event-name → web_sys type mappings. Generates
/// `event_type_matches()` and the `HtmlEventConverter` impl for `WebEventConverter`.
///
/// Each event entry takes one of two forms:
///
/// Default conversion:
/// ```ignore
/// #[events = name, ...]
/// #[event_type = web_sys::Type]
/// fn converter(event: &PlatformEventData) -> ReturnType;
/// ```
///
/// Custom conversion:
/// ```ignore
/// #[events = name, ...]
/// #[event_type = web_sys::Type]
/// fn converter(event: &PlatformEventData) -> ReturnType { body }
/// ```
macro_rules! web_events {
    (
        $(
            #[events = $($name:ident),+]
            #[event_type = $ws:ty]
            fn $conv:ident ( $evt:ident : $evt_ty:ty ) -> $ret:ty $( $body:block )?;
        )+
    ) => {
        pub(crate) fn event_type_matches(name: &str, event: &web_sys::Event) -> bool {
            let m = match name {
                $( $(stringify!($name))|+ => event.is_instance_of::<$ws>(), )+
                _ => true,
            };
            if !m {
                tracing::warn!("Ignoring \"{name}\": not the expected type: {event:?}");
            }
            m
        }

        impl HtmlEventConverter for WebEventConverter {
            $( web_events!(@method $ws, $conv, $evt -> $ret $(, $body)?); )+
        }
    };

    // Default conversion: construct Synthetic directly via unchecked_into
    (@method $ws:ty, $conv:ident, $evt:ident -> $ret:ty) => {
        #[inline(always)]
        fn $conv(&self, $evt: &PlatformEventData) -> $ret {
            Synthetic::new(downcast_event($evt).raw.clone().unchecked_into::<$ws>()).into()
        }
    };

    // Custom conversion body
    (@method $ws:ty, $conv:ident, $evt:ident -> $ret:ty, $body:block) => {
        #[inline(always)]
        fn $conv(&self, $evt: &PlatformEventData) -> $ret $body
    };
}

web_events! {
    #[events = animationstart, animationend, animationiteration]
    #[event_type = web_sys::AnimationEvent]
    fn convert_animation_data(event: &PlatformEventData) -> dioxus_html::AnimationData;

    #[events = cancel]
    #[event_type = web_sys::Event]
    fn convert_cancel_data(event: &PlatformEventData) -> dioxus_html::CancelData;

    #[events = copy, cut, paste]
    #[event_type = web_sys::Event]
    fn convert_clipboard_data(event: &PlatformEventData) -> dioxus_html::ClipboardData;

    #[events = compositionend, compositionstart, compositionupdate]
    #[event_type = web_sys::CompositionEvent]
    fn convert_composition_data(event: &PlatformEventData) -> dioxus_html::CompositionData;

    #[events = drag, dragend, dragenter, dragexit, dragleave,
               dragover, dragstart, drop]
    #[event_type = web_sys::DragEvent]
    fn convert_drag_data(event: &PlatformEventData) -> DragData {
        let event = downcast_event(event);
        DragData::new(Synthetic::new(
            event.raw.clone().unchecked_into::<web_sys::DragEvent>(),
        ))
    };

    #[events = blur, focus, focusin, focusout]
    #[event_type = web_sys::FocusEvent]
    fn convert_focus_data(event: &PlatformEventData) -> dioxus_html::FocusData;

    #[events = change, input, invalid, reset, submit]
    #[event_type = web_sys::Event]
    fn convert_form_data(event: &PlatformEventData) -> FormData {
        let event = downcast_event(event);
        FormData::new(WebFormData::new(event.element.clone(), event.raw.clone()))
    };

    #[events = error, load]
    #[event_type = web_sys::Event]
    fn convert_image_data(event: &PlatformEventData) -> ImageData {
        let event = downcast_event(event);
        ImageData::new(WebImageEvent::new(event.raw.clone(), event.raw.type_() == "error"))
    };

    #[events = keydown, keyup, keypress]
    #[event_type = web_sys::KeyboardEvent]
    fn convert_keyboard_data(event: &PlatformEventData) -> dioxus_html::KeyboardData;

    #[events = abort, canplay, canplaythrough, durationchange, emptied,
               encrypted, ended, loadeddata, loadedmetadata, loadstart,
               pause, play, playing, progress, ratechange, seeked,
               seeking, stalled, suspend, timeupdate, volumechange,
               waiting]
    #[event_type = web_sys::Event]
    fn convert_media_data(event: &PlatformEventData) -> dioxus_html::MediaData;

    #[events = mounted]
    #[event_type = web_sys::Element]
    fn convert_mounted_data(event: &PlatformEventData) -> MountedData {
        #[cfg(feature = "mounted")]
        {
            Synthetic::new(
                event
                    .downcast::<web_sys::Element>()
                    .expect("event should be a web_sys::Element")
                    .clone(),
            )
            .into()
        }
        #[cfg(not(feature = "mounted"))]
        {
            let _ = event;
            panic!("mounted events require the `mounted` feature on dioxus-web")
        }
    };

    #[events = click, contextmenu, dblclick, doubleclick,
               mousedown, mouseenter, mouseleave, mousemove,
               mouseout, mouseover, mouseup]
    #[event_type = web_sys::MouseEvent]
    fn convert_mouse_data(event: &PlatformEventData) -> dioxus_html::MouseData;

    #[events = pointerdown, pointermove, pointerup, pointerover,
               pointerout, pointerenter, pointerleave,
               gotpointercapture, lostpointercapture,
               pointerlockchange, pointerlockerror, auxclick]
    #[event_type = web_sys::PointerEvent]
    fn convert_pointer_data(event: &PlatformEventData) -> dioxus_html::PointerData;

    #[events = resize]
    #[event_type = web_sys::ResizeObserverEntry]
    fn convert_resize_data(event: &PlatformEventData) -> dioxus_html::ResizeData;

    #[events = scroll, scrollend]
    #[event_type = web_sys::Event]
    fn convert_scroll_data(event: &PlatformEventData) -> dioxus_html::ScrollData;

    #[events = select, selectstart, selectionchange]
    #[event_type = web_sys::Event]
    fn convert_selection_data(event: &PlatformEventData) -> dioxus_html::SelectionData;

    #[events = toggle, beforetoggle]
    #[event_type = web_sys::Event]
    fn convert_toggle_data(event: &PlatformEventData) -> dioxus_html::ToggleData;

    #[events = touchcancel, touchend, touchmove, touchstart]
    #[event_type = web_sys::TouchEvent]
    fn convert_touch_data(event: &PlatformEventData) -> dioxus_html::TouchData;

    #[events = transitionend]
    #[event_type = web_sys::TransitionEvent]
    fn convert_transition_data(event: &PlatformEventData) -> dioxus_html::TransitionData;

    #[events = visible]
    #[event_type = web_sys::IntersectionObserverEntry]
    fn convert_visible_data(event: &PlatformEventData) -> dioxus_html::VisibleData;

    #[events = wheel]
    #[event_type = web_sys::WheelEvent]
    fn convert_wheel_data(event: &PlatformEventData) -> dioxus_html::WheelData;
}

/// A extension trait for web-sys events that provides a way to get the event as a web-sys event.
pub trait WebEventExt {
    /// The web specific event type
    type WebEvent;

    /// Try to downcast this event as a `web-sys` event.
    fn try_as_web_event(&self) -> Option<Self::WebEvent>;

    /// Downcast this event as a `web-sys` event.
    #[inline(always)]
    fn as_web_event(&self) -> Self::WebEvent
    where
        Self::WebEvent: 'static,
    {
        self.try_as_web_event().unwrap_or_else(|| {
            panic!(
                "Error downcasting to `web-sys`, event should be a {}.",
                std::any::type_name::<Self::WebEvent>()
            )
        })
    }
}

struct GenericWebSysEvent {
    raw: Event,
    element: Element,
}

// todo: some of these events are being casted to the wrong event type.
// We need tests that simulate clicks/etc and make sure every event type works.
pub(crate) fn virtual_event_from_websys_event(
    event: web_sys::Event,
    target: Element,
) -> PlatformEventData {
    PlatformEventData::new(Box::new(GenericWebSysEvent {
        raw: event,
        element: target,
    }))
}

pub(crate) fn load_document() -> Document {
    web_sys::window()
        .expect("should have access to the Window")
        .document()
        .expect("should have access to the Document")
}
