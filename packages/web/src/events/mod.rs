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

impl HtmlEventConverter for WebEventConverter {
    #[inline(always)]
    fn convert_animation_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::AnimationData {
        Synthetic::<web_sys::AnimationEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_cancel_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::CancelData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_clipboard_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ClipboardData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_composition_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::CompositionData {
        Synthetic::<web_sys::CompositionEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_drag_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::DragData {
        let event = downcast_event(event);
        DragData::new(Synthetic::new(
            event.raw.clone().unchecked_into::<web_sys::DragEvent>(),
        ))
    }

    #[inline(always)]
    fn convert_focus_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::FocusData {
        Synthetic::<web_sys::FocusEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_form_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::FormData {
        let event = downcast_event(event);
        FormData::new(WebFormData::new(event.element.clone(), event.raw.clone()))
    }

    #[inline(always)]
    fn convert_image_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::ImageData {
        let event = downcast_event(event);
        let error = event.raw.type_() == "error";
        ImageData::new(WebImageEvent::new(event.raw.clone(), error))
    }

    #[inline(always)]
    fn convert_keyboard_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::KeyboardData {
        Synthetic::<web_sys::KeyboardEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_media_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::MediaData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[allow(unused_variables)]
    #[inline(always)]
    fn convert_mounted_data(&self, event: &dioxus_html::PlatformEventData) -> MountedData {
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
            panic!("mounted events are not supported without the mounted feature on the dioxus-web crate enabled")
        }
    }

    #[inline(always)]
    fn convert_mouse_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::MouseData {
        Synthetic::<web_sys::MouseEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_pointer_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::PointerData {
        Synthetic::<web_sys::PointerEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_resize_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ResizeData {
        Synthetic::<web_sys::ResizeObserverEntry>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_scroll_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ScrollData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_selection_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::SelectionData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_toggle_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ToggleData {
        Synthetic::new(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_touch_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::TouchData {
        Synthetic::<web_sys::TouchEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_transition_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::TransitionData {
        Synthetic::<web_sys::TransitionEvent>::from(downcast_event(event).raw.clone()).into()
    }

    #[inline(always)]
    fn convert_visible_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::VisibleData {
        Synthetic::<web_sys::IntersectionObserverEntry>::from(downcast_event(event).raw.clone())
            .into()
    }

    #[inline(always)]
    fn convert_wheel_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::WheelData {
        Synthetic::<web_sys::WheelEvent>::from(downcast_event(event).raw.clone()).into()
    }
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

macro_rules! uncheck_convert {
    ($t:ty) => {
        impl From<Event> for Synthetic<$t> {
            #[inline]
            fn from(e: Event) -> Self {
                let e: $t = e.unchecked_into();
                Self::new(e)
            }
        }

        impl From<&Event> for Synthetic<$t> {
            #[inline]
            fn from(e: &Event) -> Self {
                let e: &$t = e.unchecked_ref();
                Self::new(e.clone())
            }
        }
    };
    ($($t:ty),+ $(,)?) => {
        $(uncheck_convert!($t);)+
    };
}

uncheck_convert![
    web_sys::CompositionEvent,
    web_sys::KeyboardEvent,
    web_sys::TouchEvent,
    web_sys::PointerEvent,
    web_sys::WheelEvent,
    web_sys::AnimationEvent,
    web_sys::TransitionEvent,
    web_sys::MouseEvent,
    web_sys::FocusEvent,
];
