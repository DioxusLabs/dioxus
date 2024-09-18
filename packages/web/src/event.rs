use std::{any::Any, collections::HashMap};

use dioxus_html::{
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    DragData, FormData, FormValue, HasDragData, HasFileData, HasFormData, HasImageData,
    HasMouseData, HtmlEventConverter, ImageData, MountedData, PlatformEventData, ScrollData,
};
use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::{Document, Element, Event, MouseEvent};

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
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_clipboard_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ClipboardData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_composition_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::CompositionData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_drag_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::DragData {
        let event = downcast_event(event);
        DragData::new(WebDragData::new(event.raw.clone().unchecked_into()))
    }

    #[inline(always)]
    fn convert_focus_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::FocusData {
        downcast_event(event).raw.clone().into()
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
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_media_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::MediaData {
        downcast_event(event).raw.clone().into()
    }

    #[allow(unused_variables)]
    #[inline(always)]
    fn convert_mounted_data(&self, event: &dioxus_html::PlatformEventData) -> MountedData {
        #[cfg(feature = "mounted")]
        {
            MountedData::from(
                event
                    .downcast::<web_sys::Element>()
                    .expect("event should be a web_sys::Element"),
            )
        }
        #[cfg(not(feature = "mounted"))]
        {
            panic!("mounted events are not supported without the mounted feature on the dioxus-web crate enabled")
        }
    }

    #[inline(always)]
    fn convert_mouse_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::MouseData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_pointer_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::PointerData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_resize_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ResizeData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_scroll_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ScrollData {
        ScrollData::from(downcast_event(event).raw.clone())
    }

    #[inline(always)]
    fn convert_selection_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::SelectionData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_toggle_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::ToggleData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_touch_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::TouchData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_transition_data(
        &self,
        event: &dioxus_html::PlatformEventData,
    ) -> dioxus_html::TransitionData {
        downcast_event(event).raw.clone().into()
    }

    #[inline(always)]
    fn convert_wheel_data(&self, event: &dioxus_html::PlatformEventData) -> dioxus_html::WheelData {
        downcast_event(event).raw.clone().into()
    }
}

/// A extension trait for web-sys events that provides a way to get the event as a web-sys event.
pub trait WebEventExt<E> {
    /// Try to downcast this event as a `web-sys` event.
    fn try_as_web_event(&self) -> Option<E>;

    /// Downcast this event as a `web-sys` event.
    #[inline(always)]
    fn as_web_event(&self) -> E
    where
        E: 'static,
    {
        self.try_as_web_event().unwrap_or_else(|| {
            panic!(
                "Error downcasting to `web-sys`, event should be a {}.",
                std::any::type_name::<E>()
            )
        })
    }
}

impl WebEventExt<web_sys::AnimationEvent> for dioxus_html::AnimationData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::AnimationEvent> {
        self.downcast::<web_sys::AnimationEvent>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::ClipboardData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<web_sys::CompositionEvent> for dioxus_html::CompositionData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::CompositionEvent> {
        self.downcast::<web_sys::CompositionEvent>().cloned()
    }
}

impl WebEventExt<web_sys::MouseEvent> for dioxus_html::DragData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::MouseEvent> {
        self.downcast::<WebDragData>()
            .map(|data| &data.raw)
            .cloned()
    }
}

impl WebEventExt<web_sys::FocusEvent> for dioxus_html::FocusData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::FocusEvent> {
        self.downcast::<web_sys::FocusEvent>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::FormData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<WebImageEvent> for dioxus_html::ImageData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<WebImageEvent> {
        self.downcast::<WebImageEvent>().cloned()
    }
}

impl WebEventExt<web_sys::KeyboardEvent> for dioxus_html::KeyboardData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::KeyboardEvent> {
        self.downcast::<web_sys::KeyboardEvent>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::MediaData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<web_sys::Element> for MountedData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Element> {
        self.downcast::<web_sys::Element>().cloned()
    }
}

impl WebEventExt<web_sys::MouseEvent> for dioxus_html::MouseData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::MouseEvent> {
        self.downcast::<web_sys::MouseEvent>().cloned()
    }
}

impl WebEventExt<web_sys::PointerEvent> for dioxus_html::PointerData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::PointerEvent> {
        self.downcast::<web_sys::PointerEvent>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for ScrollData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::SelectionData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::ToggleData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::Event> {
        self.downcast::<web_sys::Event>().cloned()
    }
}

impl WebEventExt<web_sys::TouchEvent> for dioxus_html::TouchData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::TouchEvent> {
        self.downcast::<web_sys::TouchEvent>().cloned()
    }
}

impl WebEventExt<web_sys::TransitionEvent> for dioxus_html::TransitionData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::TransitionEvent> {
        self.downcast::<web_sys::TransitionEvent>().cloned()
    }
}

impl WebEventExt<web_sys::WheelEvent> for dioxus_html::WheelData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::WheelEvent> {
        self.downcast::<web_sys::WheelEvent>().cloned()
    }
}

impl WebEventExt<web_sys::ResizeObserverEntry> for dioxus_html::ResizeData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::ResizeObserverEntry> {
        self.downcast::<web_sys::CustomEvent>()
            .and_then(|e| e.detail().dyn_into::<web_sys::ResizeObserverEntry>().ok())
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

#[derive(Clone)]
struct WebImageEvent {
    raw: Event,
    error: bool,
}

impl WebImageEvent {
    fn new(raw: Event, error: bool) -> Self {
        Self { raw, error }
    }
}

impl HasImageData for WebImageEvent {
    fn load_error(&self) -> bool {
        self.error
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw as &dyn Any
    }
}

struct WebFormData {
    element: Element,
    raw: Event,
}

impl WebFormData {
    fn new(element: Element, raw: Event) -> Self {
        Self { element, raw }
    }
}

impl HasFormData for WebFormData {
    fn value(&self) -> String {
        let target = &self.element;
        target
        .dyn_ref()
        .map(|input: &web_sys::HtmlInputElement| {
            // todo: special case more input types
            match input.type_().as_str() {
                "checkbox" => {
                    match input.checked() {
                        true => "true".to_string(),
                        false => "false".to_string(),
                    }
                },
                _ => {
                    input.value()
                }
            }
        })
        .or_else(|| {
            target
                .dyn_ref()
                .map(|input: &web_sys::HtmlTextAreaElement| input.value())
        })
        // select elements are NOT input events - because - why woudn't they be??
        .or_else(|| {
            target
                .dyn_ref()
                .map(|input: &web_sys::HtmlSelectElement| input.value())
        })
        .or_else(|| {
            target
                .dyn_ref::<web_sys::HtmlElement>()
                .unwrap()
                .text_content()
        })
        .expect("only an InputElement or TextAreaElement or an element with contenteditable=true can have an oninput event listener")
    }

    fn values(&self) -> HashMap<String, FormValue> {
        let mut values = HashMap::new();

        fn insert_value(map: &mut HashMap<String, FormValue>, key: String, new_value: String) {
            map.entry(key.clone()).or_default().0.push(new_value);
        }

        // try to fill in form values
        if let Some(form) = self.element.dyn_ref::<web_sys::HtmlFormElement>() {
            let form_data = get_form_data(form);
            for value in form_data.entries().into_iter().flatten() {
                if let Ok(array) = value.dyn_into::<Array>() {
                    if let Some(name) = array.get(0).as_string() {
                        if let Ok(item_values) = array.get(1).dyn_into::<Array>() {
                            item_values
                                .iter()
                                .filter_map(|v| v.as_string())
                                .for_each(|v| insert_value(&mut values, name.clone(), v));
                        } else if let Ok(item_value) = array.get(1).dyn_into::<JsValue>() {
                            insert_value(&mut values, name, item_value.as_string().unwrap());
                        }
                    }
                }
            }
        } else if let Some(select) = self.element.dyn_ref::<web_sys::HtmlSelectElement>() {
            // try to fill in select element values
            let options = get_select_data(select);
            values.insert("options".to_string(), FormValue(options));
        }

        values
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw as &dyn Any
    }
}

impl HasFileData for WebFormData {
    #[cfg(feature = "file_engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        let files = self
            .element
            .dyn_ref()
            .and_then(|input: &web_sys::HtmlInputElement| {
                input.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    dioxus_html::WebFileEngine::new(files).map(|f| {
                        std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                    })
                })
            });

        files
    }
}

struct WebDragData {
    raw: MouseEvent,
}

impl WebDragData {
    fn new(raw: MouseEvent) -> Self {
        Self { raw }
    }
}

impl HasDragData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.raw as &dyn std::any::Any
    }
}

impl HasMouseData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.raw as &dyn std::any::Any
    }
}

impl PointerInteraction for WebDragData {
    fn trigger_button(&self) -> Option<dioxus_html::input_data::MouseButton> {
        self.raw.trigger_button()
    }

    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        self.raw.held_buttons()
    }
}

impl ModifiersInteraction for WebDragData {
    fn modifiers(&self) -> dioxus_html::prelude::Modifiers {
        self.raw.modifiers()
    }
}

impl InteractionElementOffset for WebDragData {
    fn coordinates(&self) -> dioxus_html::geometry::Coordinates {
        self.raw.coordinates()
    }

    fn element_coordinates(&self) -> dioxus_html::geometry::ElementPoint {
        self.raw.element_coordinates()
    }
}

impl InteractionLocation for WebDragData {
    fn client_coordinates(&self) -> dioxus_html::geometry::ClientPoint {
        self.raw.client_coordinates()
    }

    fn screen_coordinates(&self) -> dioxus_html::geometry::ScreenPoint {
        self.raw.screen_coordinates()
    }

    fn page_coordinates(&self) -> dioxus_html::geometry::PagePoint {
        self.raw.page_coordinates()
    }
}

impl HasFileData for WebDragData {
    #[cfg(feature = "file_engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        let files = self
            .raw
            .dyn_ref::<web_sys::DragEvent>()
            .and_then(|drag_event| {
                drag_event.data_transfer().and_then(|dt| {
                    dt.files().and_then(|files| {
                        #[allow(clippy::arc_with_non_send_sync)]
                        dioxus_html::WebFileEngine::new(files).map(|f| {
                            std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                        })
                    })
                })
            });

        files
    }
}

// web-sys does not expose the keys api for form data, so we need to manually bind to it
#[wasm_bindgen(inline_js = r#"
export function get_form_data(form) {
    let values = new Map();
    const formData = new FormData(form);

    for (let name of formData.keys()) {
        values.set(name, formData.getAll(name));
    }

    return values;
}
"#)]
extern "C" {
    fn get_form_data(form: &web_sys::HtmlFormElement) -> js_sys::Map;
}

// web-sys does not expose the keys api for select data, so we need to manually bind to it
#[wasm_bindgen(inline_js = r#"
export function get_select_data(select) {
    let values = [];
    for (let i = 0; i < select.options.length; i++) {
      let option = select.options[i];
      if (option.selected) {
        values.push(option.value.toString());
      }
    }

    return values;
}
"#)]
extern "C" {
    fn get_select_data(select: &web_sys::HtmlSelectElement) -> Vec<String>;
}
