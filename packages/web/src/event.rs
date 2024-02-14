use std::{any::Any, collections::HashMap};

use dioxus_html::{
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    prelude::FormValue,
    DragData, FileEngine, FormData, HasDragData, HasFileData, HasFormData, HasImageData,
    HasMouseData, HtmlEventConverter, ImageData, MountedData, PlatformEventData, ScrollData,
};
use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::{Document, DragEvent, Element, Event, MouseEvent};

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
    /// Get the event as a web-sys event.
    fn web_event(&self) -> &E;
}

impl WebEventExt<web_sys::AnimationEvent> for dioxus_html::AnimationData {
    fn web_event(&self) -> &web_sys::AnimationEvent {
        self.downcast::<web_sys::AnimationEvent>()
            .expect("event should be a WebAnimationEvent")
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::ClipboardData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a web_sys::Event")
    }
}

impl WebEventExt<web_sys::CompositionEvent> for dioxus_html::CompositionData {
    fn web_event(&self) -> &web_sys::CompositionEvent {
        self.downcast::<web_sys::CompositionEvent>()
            .expect("event should be a WebCompositionEvent")
    }
}

impl WebEventExt<web_sys::MouseEvent> for dioxus_html::DragData {
    fn web_event(&self) -> &web_sys::MouseEvent {
        &self
            .downcast::<WebDragData>()
            .expect("event should be a WebMouseEvent")
            .raw
    }
}

impl WebEventExt<web_sys::FocusEvent> for dioxus_html::FocusData {
    fn web_event(&self) -> &web_sys::FocusEvent {
        self.downcast::<web_sys::FocusEvent>()
            .expect("event should be a WebFocusEvent")
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::FormData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a WebFormData")
    }
}

impl WebEventExt<WebImageEvent> for dioxus_html::ImageData {
    fn web_event(&self) -> &WebImageEvent {
        self.downcast::<WebImageEvent>()
            .expect("event should be a WebImageEvent")
    }
}

impl WebEventExt<web_sys::KeyboardEvent> for dioxus_html::KeyboardData {
    fn web_event(&self) -> &web_sys::KeyboardEvent {
        self.downcast::<web_sys::KeyboardEvent>()
            .expect("event should be a WebKeyboardEvent")
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::MediaData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a WebMediaEvent")
    }
}

impl WebEventExt<web_sys::Element> for MountedData {
    fn web_event(&self) -> &web_sys::Element {
        self.downcast::<web_sys::Element>()
            .expect("event should be a web_sys::Element")
    }
}

impl WebEventExt<web_sys::MouseEvent> for dioxus_html::MouseData {
    fn web_event(&self) -> &web_sys::MouseEvent {
        self.downcast::<web_sys::MouseEvent>()
            .expect("event should be a WebMouseEvent")
    }
}

impl WebEventExt<web_sys::PointerEvent> for dioxus_html::PointerData {
    fn web_event(&self) -> &web_sys::PointerEvent {
        self.downcast::<web_sys::PointerEvent>()
            .expect("event should be a WebPointerEvent")
    }
}

impl WebEventExt<web_sys::Event> for ScrollData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a WebScrollEvent")
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::SelectionData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a WebSelectionEvent")
    }
}

impl WebEventExt<web_sys::Event> for dioxus_html::ToggleData {
    fn web_event(&self) -> &web_sys::Event {
        self.downcast::<web_sys::Event>()
            .expect("event should be a WebToggleEvent")
    }
}

impl WebEventExt<web_sys::TouchEvent> for dioxus_html::TouchData {
    fn web_event(&self) -> &web_sys::TouchEvent {
        self.downcast::<web_sys::TouchEvent>()
            .expect("event should be a WebTouchEvent")
    }
}

impl WebEventExt<web_sys::TransitionEvent> for dioxus_html::TransitionData {
    fn web_event(&self) -> &web_sys::TransitionEvent {
        self.downcast::<web_sys::TransitionEvent>()
            .expect("event should be a WebTransitionEvent")
    }
}

impl WebEventExt<web_sys::WheelEvent> for dioxus_html::WheelData {
    fn web_event(&self) -> &web_sys::WheelEvent {
        self.downcast::<web_sys::WheelEvent>()
            .expect("event should be a WebWheelEvent")
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
            match map.entry(key) {
                std::collections::hash_map::Entry::Occupied(mut o) => {
                    let first_value = match o.get_mut() {
                        FormValue::Text(data) => std::mem::take(data),
                        FormValue::VecText(vec) => {
                            vec.push(new_value);
                            return;
                        }
                    };
                    let _ = o.insert(FormValue::VecText(vec![first_value, new_value]));
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    let _ = v.insert(FormValue::Text(new_value));
                }
            }
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
            values.insert("options".to_string(), FormValue::VecText(options));
        }

        values
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw as &dyn Any
    }
}

impl HasFileData for WebFormData {
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        #[cfg(not(feature = "file_engine"))]
        let files = None;
        #[cfg(feature = "file_engine")]
        let files = self
            .element
            .dyn_ref()
            .and_then(|input: &web_sys::HtmlInputElement| {
                input.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    crate::file_engine::WebFileEngine::new(files).map(|f| {
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
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        #[cfg(not(feature = "file_engine"))]
        let files = None;
        #[cfg(feature = "file_engine")]
        let files = self.raw.dyn_ref::<DragEvent>().and_then(|drag_event| {
            drag_event.data_transfer().and_then(|dt| {
                dt.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    crate::file_engine::WebFileEngine::new(files).map(|f| {
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
