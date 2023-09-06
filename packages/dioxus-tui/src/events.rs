use core::panic;

use dioxus_html::*;

use crate::element::TuiElement;

fn downcast(event: &PlatformEventData) -> plasmo::EventData {
    event
        .downcast::<plasmo::EventData>()
        .expect("event should be of type EventData")
        .clone()
}

pub(crate) struct SerializedHtmlEventConverter;

impl HtmlEventConverter for SerializedHtmlEventConverter {
    fn convert_animation_data(&self, _: &PlatformEventData) -> AnimationData {
        panic!("animation events not supported")
    }

    fn convert_clipboard_data(&self, _: &PlatformEventData) -> ClipboardData {
        panic!("clipboard events not supported")
    }

    fn convert_composition_data(&self, _: &PlatformEventData) -> CompositionData {
        panic!("composition events not supported")
    }

    fn convert_drag_data(&self, _: &PlatformEventData) -> DragData {
        panic!("drag events not supported")
    }

    fn convert_focus_data(&self, event: &PlatformEventData) -> FocusData {
        if let plasmo::EventData::Focus(event) = downcast(event) {
            FocusData::new(event)
        } else {
            panic!("event should be of type Focus")
        }
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        if let plasmo::EventData::Form(event) = downcast(event) {
            FormData::new(event)
        } else {
            panic!("event should be of type Form")
        }
    }

    fn convert_image_data(&self, _: &PlatformEventData) -> ImageData {
        panic!("image events not supported")
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        if let plasmo::EventData::Keyboard(event) = downcast(event) {
            KeyboardData::new(event)
        } else {
            panic!("event should be of type Keyboard")
        }
    }

    fn convert_media_data(&self, _: &PlatformEventData) -> MediaData {
        panic!("media events not supported")
    }

    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData {
        event.downcast::<TuiElement>().cloned().unwrap().into()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        if let plasmo::EventData::Mouse(event) = downcast(event) {
            MouseData::new(event)
        } else {
            panic!("event should be of type Mouse")
        }
    }

    fn convert_pointer_data(&self, _: &PlatformEventData) -> PointerData {
        panic!("pointer events not supported")
    }

    fn convert_scroll_data(&self, _: &PlatformEventData) -> ScrollData {
        panic!("scroll events not supported")
    }

    fn convert_selection_data(&self, _: &PlatformEventData) -> SelectionData {
        panic!("selection events not supported")
    }

    fn convert_toggle_data(&self, _: &PlatformEventData) -> ToggleData {
        panic!("toggle events not supported")
    }

    fn convert_touch_data(&self, _: &PlatformEventData) -> TouchData {
        panic!("touch events not supported")
    }

    fn convert_transition_data(&self, _: &PlatformEventData) -> TransitionData {
        panic!("transition events not supported")
    }

    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData {
        if let plasmo::EventData::Wheel(event) = downcast(event) {
            WheelData::new(event)
        } else {
            panic!("event should be of type Wheel")
        }
    }
}
