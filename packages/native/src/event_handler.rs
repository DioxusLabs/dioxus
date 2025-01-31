use std::collections::HashMap;

use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    AnimationData, ClipboardData, CompositionData, DragData, FocusData, FormData, FormValue,
    HasFileData, HasFormData, HasMouseData, HtmlEventConverter, ImageData, KeyboardData, MediaData,
    MountedData, MouseData, PlatformEventData, PointerData, ResizeData, ScrollData, SelectionData,
    ToggleData, TouchData, TransitionData, VisibleData, WheelData,
};
use keyboard_types::Modifiers;

use super::keyboard_event::BlitzKeyboardData;

#[derive(Clone)]
pub struct NativeClickData;

impl InteractionLocation for NativeClickData {
    fn client_coordinates(&self) -> ClientPoint {
        todo!()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        todo!()
    }

    fn page_coordinates(&self) -> PagePoint {
        todo!()
    }
}
impl InteractionElementOffset for NativeClickData {
    fn element_coordinates(&self) -> ElementPoint {
        todo!()
    }
}
impl ModifiersInteraction for NativeClickData {
    fn modifiers(&self) -> Modifiers {
        todo!()
    }
}

impl PointerInteraction for NativeClickData {
    fn trigger_button(&self) -> Option<MouseButton> {
        todo!()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        todo!()
    }
}
impl HasMouseData for NativeClickData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        todo!()
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        todo!()
    }

    fn convert_composition_data(&self, _event: &PlatformEventData) -> CompositionData {
        todo!()
    }

    fn convert_drag_data(&self, _event: &PlatformEventData) -> DragData {
        todo!()
    }

    fn convert_focus_data(&self, _event: &PlatformEventData) -> FocusData {
        todo!()
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        let o = event.downcast::<NativeFormData>().unwrap().clone();
        FormData::from(o)
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        todo!()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        let data = event.downcast::<BlitzKeyboardData>().unwrap().clone();
        KeyboardData::from(data)
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        todo!()
    }

    fn convert_mounted_data(&self, _event: &PlatformEventData) -> MountedData {
        todo!()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        let o = event.downcast::<NativeClickData>().unwrap().clone();
        MouseData::from(o)
    }

    fn convert_pointer_data(&self, _event: &PlatformEventData) -> PointerData {
        todo!()
    }

    fn convert_scroll_data(&self, _event: &PlatformEventData) -> ScrollData {
        todo!()
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        todo!()
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        todo!()
    }

    fn convert_touch_data(&self, _event: &PlatformEventData) -> TouchData {
        todo!()
    }

    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        todo!()
    }

    fn convert_wheel_data(&self, _event: &PlatformEventData) -> WheelData {
        todo!()
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        todo!()
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
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
