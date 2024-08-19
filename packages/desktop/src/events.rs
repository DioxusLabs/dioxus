//! Convert a serialized event to an event trigger

use crate::{
    element::DesktopElement,
    file_upload::{DesktopFileDragEvent, DesktopFileUploadForm},
};
use dioxus_html::*;

pub(crate) struct SerializedHtmlEventConverter;

impl HtmlEventConverter for SerializedHtmlEventConverter {
    fn convert_animation_data(&self, event: &PlatformEventData) -> AnimationData {
        event
            .downcast::<SerializedAnimationData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_clipboard_data(&self, event: &PlatformEventData) -> ClipboardData {
        event
            .downcast::<SerializedClipboardData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_composition_data(&self, event: &PlatformEventData) -> CompositionData {
        event
            .downcast::<SerializedCompositionData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_drag_data(&self, event: &PlatformEventData) -> DragData {
        // Attempt a simple serialized data conversion
        if let Some(_data) = event.downcast::<SerializedDragData>() {
            return _data.clone().into();
        }

        // If that failed then it's a file drag form
        event
            .downcast::<DesktopFileDragEvent>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_focus_data(&self, event: &PlatformEventData) -> FocusData {
        event
            .downcast::<SerializedFocusData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        // Attempt a simple serialized form data conversion
        if let Some(_data) = event.downcast::<SerializedFormData>() {
            return _data.clone().into();
        }

        // If that failed then it's a file upload form
        event
            .downcast::<DesktopFileUploadForm>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_image_data(&self, event: &PlatformEventData) -> ImageData {
        event
            .downcast::<SerializedImageData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        event
            .downcast::<SerializedKeyboardData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_media_data(&self, event: &PlatformEventData) -> MediaData {
        event
            .downcast::<SerializedMediaData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData {
        event.downcast::<DesktopElement>().cloned().unwrap().into()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<SerializedMouseData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData {
        event
            .downcast::<SerializedPointerData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_resize_data(&self, event: &PlatformEventData) -> ResizeData {
        event
            .downcast::<SerializedResizeData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_scroll_data(&self, event: &PlatformEventData) -> ScrollData {
        event
            .downcast::<SerializedScrollData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_selection_data(&self, event: &PlatformEventData) -> SelectionData {
        event
            .downcast::<SerializedSelectionData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_toggle_data(&self, event: &PlatformEventData) -> ToggleData {
        event
            .downcast::<SerializedToggleData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_touch_data(&self, event: &PlatformEventData) -> TouchData {
        event
            .downcast::<SerializedTouchData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_transition_data(&self, event: &PlatformEventData) -> TransitionData {
        event
            .downcast::<SerializedTransitionData>()
            .cloned()
            .unwrap()
            .into()
    }

    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData {
        event
            .downcast::<SerializedWheelData>()
            .cloned()
            .unwrap()
            .into()
    }
}
