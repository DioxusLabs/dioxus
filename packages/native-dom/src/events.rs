use blitz_traits::events::{BlitzKeyEvent, BlitzMouseButtonEvent, MouseEventButton};
use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    AnimationData, CancelData, ClipboardData, CompositionData, DragData, FocusData, FormData,
    FormValue, HasFileData, HasFocusData, HasFormData, HasKeyboardData, HasMouseData,
    HtmlEventConverter, ImageData, KeyboardData, MediaData, MountedData, MouseData,
    PlatformEventData, PointerData, ResizeData, ScrollData, SelectionData, ToggleData, TouchData,
    TransitionData, VisibleData, WheelData,
};
use keyboard_types::{Code, Key, Location, Modifiers};
use std::any::Any;

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_cancel_data(&self, _event: &PlatformEventData) -> CancelData {
        unimplemented!("todo: convert_cancel_data in dioxus-native. requires support in blitz")
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        event.downcast::<NativeFormData>().unwrap().clone().into()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event.downcast::<NativeClickData>().unwrap().clone().into()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        event
            .downcast::<BlitzKeyboardData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_focus_data(&self, _event: &PlatformEventData) -> FocusData {
        NativeFocusData {}.into()
    }

    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        unimplemented!("todo: convert_animation_data in dioxus-native. requires support in blitz")
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        unimplemented!("todo: convert_clipboard_data in dioxus-native. requires support in blitz")
    }

    fn convert_composition_data(&self, _event: &PlatformEventData) -> CompositionData {
        unimplemented!("todo: convert_composition_data in dioxus-native. requires support in blitz")
    }

    fn convert_drag_data(&self, _event: &PlatformEventData) -> DragData {
        unimplemented!("todo: convert_drag_data in dioxus-native. requires support in blitz")
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        unimplemented!("todo: convert_image_data in dioxus-native. requires support in blitz")
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        unimplemented!("todo: convert_media_data in dioxus-native. requires support in blitz")
    }

    fn convert_mounted_data(&self, _event: &PlatformEventData) -> MountedData {
        unimplemented!("todo: convert_mounted_data in dioxus-native. requires support in blitz")
    }

    fn convert_pointer_data(&self, _event: &PlatformEventData) -> PointerData {
        unimplemented!("todo: convert_pointer_data in dioxus-native. requires support in blitz")
    }

    fn convert_scroll_data(&self, _event: &PlatformEventData) -> ScrollData {
        unimplemented!("todo: convert_scroll_data in dioxus-native. requires support in blitz")
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        unimplemented!("todo: convert_selection_data in dioxus-native. requires support in blitz")
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        unimplemented!("todo: convert_toggle_data in dioxus-native. requires support in blitz")
    }

    fn convert_touch_data(&self, _event: &PlatformEventData) -> TouchData {
        unimplemented!("todo: convert_touch_data in dioxus-native. requires support in blitz")
    }

    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        unimplemented!("todo: convert_transition_data in dioxus-native. requires support in blitz")
    }

    fn convert_wheel_data(&self, _event: &PlatformEventData) -> WheelData {
        unimplemented!("todo: convert_wheel_data in dioxus-native. requires support in blitz")
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        unimplemented!("todo: convert_resize_data in dioxus-native. requires support in blitz")
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
        unimplemented!("todo: convert_visible_data in dioxus-native. requires support in blitz")
    }
}

#[derive(Clone, Debug)]
pub struct NativeFormData {
    pub value: String,
    pub values: Vec<(String, FormValue)>,
}

impl HasFormData for NativeFormData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        self.values.clone()
    }
    fn valid(&self) -> bool {
        // todo: actually implement validation here.
        true
    }
}

impl HasFileData for NativeFormData {
    fn files(&self) -> Vec<dioxus_html::FileData> {
        vec![]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlitzKeyboardData(pub(crate) BlitzKeyEvent);

impl ModifiersInteraction for BlitzKeyboardData {
    fn modifiers(&self) -> Modifiers {
        self.0.modifiers
    }
}

impl HasKeyboardData for BlitzKeyboardData {
    fn key(&self) -> Key {
        self.0.key.clone()
    }

    fn code(&self) -> Code {
        self.0.code
    }

    fn location(&self) -> Location {
        self.0.location
    }

    fn is_auto_repeating(&self) -> bool {
        self.0.is_auto_repeating
    }

    fn is_composing(&self) -> bool {
        self.0.is_composing
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativeClickData(pub(crate) BlitzMouseButtonEvent);

impl InteractionLocation for NativeClickData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.x as _, self.0.y as _)
    }

    // these require blitz to pass them along, or a dom rect
    fn screen_coordinates(&self) -> ScreenPoint {
        unimplemented!()
    }

    fn page_coordinates(&self) -> PagePoint {
        unimplemented!()
    }
}

impl InteractionElementOffset for NativeClickData {
    fn element_coordinates(&self) -> ElementPoint {
        unimplemented!()
    }
}

impl ModifiersInteraction for NativeClickData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl PointerInteraction for NativeClickData {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(match self.0.button {
            MouseEventButton::Main => MouseButton::Primary,
            MouseEventButton::Auxiliary => MouseButton::Auxiliary,
            MouseEventButton::Secondary => MouseButton::Secondary,
            MouseEventButton::Fourth => MouseButton::Fourth,
            MouseEventButton::Fifth => MouseButton::Fifth,
        })
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}
impl HasMouseData for NativeClickData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}

#[derive(Clone)]
pub struct NativeFocusData {}
impl HasFocusData for NativeFocusData {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
}
