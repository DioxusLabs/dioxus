use dioxus_html::{
    DragData, FormData, HtmlEventConverter, ImageData, MountedData, PlatformEventData,
};
use drag::WebDragData;
use form::WebFormData;
use image::WebImageEvent;
use synthetic::Synthetic;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

mod drag;
mod ext;
mod file;
mod form;
mod image;
mod resize;
mod synthetic;

pub(crate) struct WebEventConverter;

impl HtmlEventConverter for WebEventConverter {
    #[inline(always)]
    fn convert_animation_data(&self, event: &PlatformEventData) -> dioxus_html::AnimationData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_clipboard_data(&self, event: &PlatformEventData) -> dioxus_html::ClipboardData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_composition_data(&self, event: &PlatformEventData) -> dioxus_html::CompositionData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_drag_data(&self, event: &PlatformEventData) -> dioxus_html::DragData {
        let event = downcast_event(event);
        DragData::new(WebDragData::new(event.raw.clone().unchecked_into()))
    }

    #[inline(always)]
    fn convert_focus_data(&self, event: &PlatformEventData) -> dioxus_html::FocusData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_form_data(&self, event: &PlatformEventData) -> dioxus_html::FormData {
        let event = downcast_event(event);
        FormData::new(WebFormData::new(event.element.clone(), event.raw.clone()))
    }

    #[inline(always)]
    fn convert_image_data(&self, event: &PlatformEventData) -> dioxus_html::ImageData {
        let event = downcast_event(event);
        let error = event.raw.type_() == "error";
        ImageData::new(WebImageEvent::new(event.raw.clone(), error))
    }

    #[inline(always)]
    fn convert_keyboard_data(&self, event: &PlatformEventData) -> dioxus_html::KeyboardData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_media_data(&self, event: &PlatformEventData) -> dioxus_html::MediaData {
        event.synthesize()
    }

    #[allow(unused_variables)]
    #[inline(always)]
    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData {
        #[cfg(feature = "mounted")]
        {
            MountedData::new(Synthetic::new(
                event
                    .downcast::<web_sys::Element>()
                    .expect("event should be a web_sys::Element")
                    .clone(),
            ))
        }
        #[cfg(not(feature = "mounted"))]
        {
            panic!("mounted events are not supported without the mounted feature on the dioxus-web crate enabled")
        }
    }

    #[inline(always)]
    fn convert_mouse_data(&self, event: &PlatformEventData) -> dioxus_html::MouseData {
        let event = event
            .downcast::<GenericWebSysEvent>()
            .unwrap()
            .raw
            .unchecked_ref::<web_sys::MouseEvent>()
            .clone();
        Synthetic::new(event).into()
    }

    #[inline(always)]
    fn convert_pointer_data(&self, event: &PlatformEventData) -> dioxus_html::PointerData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_resize_data(&self, event: &PlatformEventData) -> dioxus_html::ResizeData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_scroll_data(&self, event: &PlatformEventData) -> dioxus_html::ScrollData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_selection_data(&self, event: &PlatformEventData) -> dioxus_html::SelectionData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_toggle_data(&self, event: &PlatformEventData) -> dioxus_html::ToggleData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_touch_data(&self, event: &PlatformEventData) -> dioxus_html::TouchData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_transition_data(&self, event: &PlatformEventData) -> dioxus_html::TransitionData {
        event.synthesize()
    }

    #[inline(always)]
    fn convert_wheel_data(&self, event: &PlatformEventData) -> dioxus_html::WheelData {
        let event = event
            .downcast::<GenericWebSysEvent>()
            .unwrap()
            .raw
            .unchecked_ref::<web_sys::WheelEvent>()
            .clone();
        Synthetic::new(event).into()
    }
}

struct GenericWebSysEvent {
    raw: Event,
    element: Element,
}
/// Converts our Synthetic wrapper into the trait objects dioxus is expectinga
trait Synthesize {
    fn synthesize<O: JsCast + 'static, F: From<Synthetic<O>>>(&self) -> F;
}

impl Synthesize for PlatformEventData {
    fn synthesize<O: JsCast + 'static, F: From<Synthetic<O>>>(&self) -> F {
        let generic = self
            .downcast::<GenericWebSysEvent>()
            .expect("event should be a GenericWebSysEvent");

        Synthetic::new(generic.raw.clone().unchecked_into()).into()
    }
}

#[inline(always)]
fn downcast_event(event: &PlatformEventData) -> &GenericWebSysEvent {
    event
        .downcast::<GenericWebSysEvent>()
        .expect("event should be a GenericWebSysEvent")
}

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
