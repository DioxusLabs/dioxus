use std::collections::HashMap;

use dioxus::{
    html::{FormValue, HasFileData, HasFormData},
    prelude::{HtmlEventConverter, PlatformEventData},
};

use super::keyboard_event::BlitzKeyboardData;

#[derive(Clone)]
pub struct NativeClickData;

impl dioxus::html::point_interaction::InteractionLocation for NativeClickData {
    fn client_coordinates(&self) -> dioxus::prelude::dioxus_elements::geometry::ClientPoint {
        todo!()
    }

    fn screen_coordinates(&self) -> dioxus::prelude::dioxus_elements::geometry::ScreenPoint {
        todo!()
    }

    fn page_coordinates(&self) -> dioxus::prelude::dioxus_elements::geometry::PagePoint {
        todo!()
    }
}
impl dioxus::html::point_interaction::InteractionElementOffset for NativeClickData {
    fn element_coordinates(&self) -> dioxus::prelude::dioxus_elements::geometry::ElementPoint {
        todo!()
    }
}
impl dioxus::html::point_interaction::ModifiersInteraction for NativeClickData {
    fn modifiers(&self) -> dioxus::prelude::Modifiers {
        todo!()
    }
}

impl dioxus::html::point_interaction::PointerInteraction for NativeClickData {
    fn trigger_button(&self) -> Option<dioxus::prelude::dioxus_elements::input_data::MouseButton> {
        todo!()
    }

    fn held_buttons(&self) -> dioxus::prelude::dioxus_elements::input_data::MouseButtonSet {
        todo!()
    }
}
impl dioxus::html::HasMouseData for NativeClickData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_animation_data(&self, _event: &PlatformEventData) -> dioxus::prelude::AnimationData {
        todo!()
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> dioxus::prelude::ClipboardData {
        todo!()
    }

    fn convert_composition_data(
        &self,
        _event: &PlatformEventData,
    ) -> dioxus::prelude::CompositionData {
        todo!()
    }

    fn convert_drag_data(&self, _event: &PlatformEventData) -> dioxus::prelude::DragData {
        todo!()
    }

    fn convert_focus_data(&self, _event: &PlatformEventData) -> dioxus::prelude::FocusData {
        todo!()
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> dioxus::prelude::FormData {
        let o = event.downcast::<NativeFormData>().unwrap().clone();
        dioxus::prelude::FormData::from(o)
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> dioxus::prelude::ImageData {
        todo!()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> dioxus::prelude::KeyboardData {
        let data = event.downcast::<BlitzKeyboardData>().unwrap().clone();
        dioxus::prelude::KeyboardData::from(data)
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> dioxus::prelude::MediaData {
        todo!()
    }

    fn convert_mounted_data(&self, _event: &PlatformEventData) -> dioxus::prelude::MountedData {
        todo!()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> dioxus::prelude::MouseData {
        let o = event.downcast::<NativeClickData>().unwrap().clone();
        dioxus::prelude::MouseData::from(o)
    }

    fn convert_pointer_data(&self, _event: &PlatformEventData) -> dioxus::prelude::PointerData {
        todo!()
    }

    fn convert_scroll_data(&self, _event: &PlatformEventData) -> dioxus::prelude::ScrollData {
        todo!()
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> dioxus::prelude::SelectionData {
        todo!()
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> dioxus::prelude::ToggleData {
        todo!()
    }

    fn convert_touch_data(&self, _event: &PlatformEventData) -> dioxus::prelude::TouchData {
        todo!()
    }

    fn convert_transition_data(
        &self,
        _event: &PlatformEventData,
    ) -> dioxus::prelude::TransitionData {
        todo!()
    }

    fn convert_wheel_data(&self, _event: &PlatformEventData) -> dioxus::prelude::WheelData {
        todo!()
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> dioxus::prelude::ResizeData {
        todo!()
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> dioxus::prelude::VisibleData {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct NativeFormData {
    pub value: String,
    pub values: HashMap<String, FormValue>,
}

impl HasFormData for NativeFormData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> HashMap<String, FormValue> {
        self.values.clone()
    }
}

impl HasFileData for NativeFormData {}
