use dioxus_html::{DragData, FormData, HtmlEventConverter, ImageData, PlatformEventData};
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

macro_rules! with_web_event_converters {
    ($macro:ident) => {
        $macro! {
            convert_animation_data(AnimationData) => web_sys::AnimationEvent;
            convert_cancel_data(CancelData) => web_sys::Event;
            convert_clipboard_data(ClipboardData) => web_sys::Event;
            convert_composition_data(CompositionData) => web_sys::CompositionEvent;
            convert_drag_data(DragData) => web_sys::DragEvent => |event| {
                let event = downcast_event(event);
                DragData::new(Synthetic::new(
                    event.raw.clone().unchecked_into::<web_sys::DragEvent>(),
                ))
            };
            convert_focus_data(FocusData) => web_sys::FocusEvent;
            convert_form_data(FormData) => web_sys::Event => |event| {
                let event = downcast_event(event);
                FormData::new(WebFormData::new(event.element.clone(), event.raw.clone()))
            };
            convert_image_data(ImageData) => web_sys::Event => |event| {
                let event = downcast_event(event);
                ImageData::new(WebImageEvent::new(
                    event.raw.clone(),
                    event.raw.type_() == "error",
                ))
            };
            convert_keyboard_data(KeyboardData) => web_sys::KeyboardEvent;
            convert_media_data(MediaData) => web_sys::Event;
            convert_mounted_data(MountedData) => web_sys::Element => |event| {
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
            convert_mouse_data(MouseData) => web_sys::MouseEvent;
            convert_pointer_data(PointerData) => web_sys::PointerEvent;
            convert_resize_data(ResizeData) => web_sys::CustomEvent => |event| {
                Synthetic::<web_sys::ResizeObserverEntry>::from(downcast_event(event).raw.clone()).into()
            };
            convert_scroll_data(ScrollData) => web_sys::Event;
            convert_selection_data(SelectionData) => web_sys::Event;
            convert_toggle_data(ToggleData) => web_sys::Event;
            convert_touch_data(TouchData) => web_sys::TouchEvent;
            convert_transition_data(TransitionData) => web_sys::TransitionEvent;
            convert_visible_data(VisibleData) => web_sys::CustomEvent => |event| {
                Synthetic::<web_sys::IntersectionObserverEntry>::from(downcast_event(event).raw.clone()).into()
            };
            convert_wheel_data(WheelData) => web_sys::WheelEvent;
        }
    };
}

macro_rules! expand_web_event_converter {
    (
        $(
            $converter:ident($data:ident) => $web_ty:ty $(=> |$event:ident| $body:block)?;
        )*
    ) => {
        macro_rules! web_event_type_matches {
            $(
                ($event_name:ident, $converter) => {
                    $event_name.is_instance_of::<$web_ty>()
                };
            )*
        }

        impl HtmlEventConverter for WebEventConverter {
            $(
                expand_web_event_converter!(@method $converter, $data, $web_ty $(, $event, $body)?);
            )*
        }
    };

    (@method $converter:ident, $data:ident, $web_ty:ty) => {
        #[inline(always)]
        fn $converter(&self, event: &PlatformEventData) -> dioxus_html::$data {
            Synthetic::new(downcast_event(event).raw.clone().unchecked_into::<$web_ty>()).into()
        }
    };

    (@method $converter:ident, $data:ident, $web_ty:ty, $event:ident, $body:block) => {
        #[inline(always)]
        fn $converter(&self, $event: &PlatformEventData) -> dioxus_html::$data $body
    };
}

with_web_event_converters!(expand_web_event_converter);

macro_rules! expand_web_event_changes {
    (
        enum Event {
            $(
                #[convert = $converter:ident]
                #[events = [
                    $(
                        $( #[$attr:meta] )*
                        $name:ident => $raw:ident,
                    )*
                ]]
                $(#[raw = [$($raw_only:ident),* $(,)?]])?
                $group:ident($data:ident),
            )*
        }
    ) => {
        pub(crate) fn event_type_matches(name: &str, event: &web_sys::Event) -> bool {
            let m = match name {
                $(
                    $( stringify!($raw) )|* $($(| stringify!($raw_only))*)? => {
                        web_event_type_matches!(event, $converter)
                    }
                )*
                _ => true,
            };
            if !m {
                tracing::warn!("Ignoring \"{name}\": not the expected type: {event:?}");
            }
            m
        }
    };
}

dioxus_html::with_html_event_groups!(expand_web_event_changes);

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
