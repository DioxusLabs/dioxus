use dioxus_html::{FormData, HtmlEventConverter, ImageData, MountedData, PlatformEventData};
use form::WebFormData;
use load::WebImageEvent;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

mod animation;
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

macro_rules! impl_safe_converter {
    (
        $(
            ($fn_name:ident, $dioxus_event:ty, $web_sys_event:ty),
        )*
    ) => {
        $(
            #[inline(always)]
            fn $fn_name(&self, event: &dioxus_html::PlatformEventData) -> $dioxus_event {
                let raw_event = &downcast_event(event).raw;
                if let Ok(event) = raw_event.clone().dyn_into::<$web_sys_event>() {
                    <$dioxus_event>::new(Synthetic::new(event))
                } else {
                    <$dioxus_event>::default()
                }
            }
        )*
    };
}

impl HtmlEventConverter for WebEventConverter {
    impl_safe_converter!(
        (
            convert_animation_data,
            dioxus_html::AnimationData,
            web_sys::AnimationEvent
        ),
        (
            convert_clipboard_data,
            dioxus_html::ClipboardData,
            web_sys::ClipboardEvent
        ),
        (
            convert_composition_data,
            dioxus_html::CompositionData,
            web_sys::CompositionEvent
        ),
        (convert_drag_data, dioxus_html::DragData, web_sys::DragEvent),
        (
            convert_focus_data,
            dioxus_html::FocusData,
            web_sys::FocusEvent
        ),
        (
            convert_keyboard_data,
            dioxus_html::KeyboardData,
            web_sys::KeyboardEvent
        ),
        (
            convert_mouse_data,
            dioxus_html::MouseData,
            web_sys::MouseEvent
        ),
        (
            convert_pointer_data,
            dioxus_html::PointerData,
            web_sys::PointerEvent
        ),
        (
            convert_touch_data,
            dioxus_html::TouchData,
            web_sys::TouchEvent
        ),
        (
            convert_transition_data,
            dioxus_html::TransitionData,
            web_sys::TransitionEvent
        ),
        (
            convert_wheel_data,
            dioxus_html::WheelData,
            web_sys::WheelEvent
        ),
    );

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
    fn convert_visible_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::VisibleData {
        Synthetic::<web_sys::IntersectionObserverEntry>::from(downcast_event(event).raw.clone())
            .into()
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

// ResizeObserverEntry and IntersectionObserverEntry need custom From implementations
// because they come from CustomEvent details and are handled in their respective modules
