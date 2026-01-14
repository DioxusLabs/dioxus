//! Desktop-specific event converter that wraps WebEventConverter.
//!
//! This module provides a custom event converter for desktop that handles
//! desktop-specific event types like DesktopFileDragEvent which include
//! native file paths.

use crate::file_upload::{DesktopFileData, DesktopFileDragEvent, DesktopFormData};
use dioxus_html::{DragData, FileData, FormData, FormValue, HtmlEventConverter, PlatformEventData};
use dioxus_web_sys_events::{GenericWebSysEvent, WebEventConverter};
use std::path::PathBuf;
use web_sys_x::wasm_bindgen::JsCast;

/// Desktop-specific event converter that wraps WebEventConverter.
///
/// This converter handles desktop-specific event types that are created
/// in the event bridge (wry_bindgen_bridge.rs) with native file data.
pub struct DesktopEventConverter {
    inner: WebEventConverter,
}

impl Default for DesktopEventConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopEventConverter {
    /// Create a new desktop event converter.
    pub fn new() -> Self {
        Self {
            inner: WebEventConverter,
        }
    }
}

impl HtmlEventConverter for DesktopEventConverter {
    fn convert_drag_data(&self, event: &PlatformEventData) -> DragData {
        // Try to downcast to DesktopFileDragEvent first (created by the bridge for drag events)
        if let Some(desktop_drag) = event.downcast::<DesktopFileDragEvent>().cloned() {
            return DragData::new(desktop_drag);
        }

        // Fall back to web-sys conversion for standard drag events without desktop-specific data
        self.inner.convert_drag_data(event)
    }

    // Delegate all other methods to inner
    fn convert_animation_data(&self, event: &PlatformEventData) -> dioxus_html::AnimationData {
        self.inner.convert_animation_data(event)
    }

    fn convert_cancel_data(&self, event: &PlatformEventData) -> dioxus_html::CancelData {
        self.inner.convert_cancel_data(event)
    }

    fn convert_clipboard_data(&self, event: &PlatformEventData) -> dioxus_html::ClipboardData {
        self.inner.convert_clipboard_data(event)
    }

    fn convert_composition_data(&self, event: &PlatformEventData) -> dioxus_html::CompositionData {
        self.inner.convert_composition_data(event)
    }

    fn convert_focus_data(&self, event: &PlatformEventData) -> dioxus_html::FocusData {
        self.inner.convert_focus_data(event)
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> dioxus_html::FormData {
        // Check for web-sys events (from wry-bindgen bridge)
        if let Some(web_event) = event.downcast::<GenericWebSysEvent>() {
            // Check if this is a file input
            if let Some(input) = web_event.element.dyn_ref::<web_sys_x::HtmlInputElement>() {
                if input.type_() == "file" {
                    // Get files from the input - the filenames contain the native paths
                    if let Some(file_list) = input.files() {
                        let mut values = Vec::new();
                        let input_name = input.name();

                        for i in 0..file_list.length() {
                            if let Some(file) = file_list.get(i) {
                                // The filename is actually the native path
                                let path = PathBuf::from(file.name());
                                if path.exists() {
                                    let file_data = FileData::new(DesktopFileData(path));
                                    values.push((
                                        input_name.clone(),
                                        FormValue::File(Some(file_data)),
                                    ));
                                }
                            }
                        }

                        if !values.is_empty() {
                            return FormData::new(DesktopFormData::new(values));
                        }
                    }
                }
            }
        }

        // Fall back to web-sys conversion
        self.inner.convert_form_data(event)
    }

    fn convert_image_data(&self, event: &PlatformEventData) -> dioxus_html::ImageData {
        self.inner.convert_image_data(event)
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> dioxus_html::KeyboardData {
        self.inner.convert_keyboard_data(event)
    }

    fn convert_media_data(&self, event: &PlatformEventData) -> dioxus_html::MediaData {
        self.inner.convert_media_data(event)
    }

    fn convert_mounted_data(&self, event: &PlatformEventData) -> dioxus_html::MountedData {
        self.inner.convert_mounted_data(event)
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> dioxus_html::MouseData {
        self.inner.convert_mouse_data(event)
    }

    fn convert_pointer_data(&self, event: &PlatformEventData) -> dioxus_html::PointerData {
        self.inner.convert_pointer_data(event)
    }

    fn convert_resize_data(&self, event: &PlatformEventData) -> dioxus_html::ResizeData {
        self.inner.convert_resize_data(event)
    }

    fn convert_scroll_data(&self, event: &PlatformEventData) -> dioxus_html::ScrollData {
        self.inner.convert_scroll_data(event)
    }

    fn convert_selection_data(&self, event: &PlatformEventData) -> dioxus_html::SelectionData {
        self.inner.convert_selection_data(event)
    }

    fn convert_toggle_data(&self, event: &PlatformEventData) -> dioxus_html::ToggleData {
        self.inner.convert_toggle_data(event)
    }

    fn convert_touch_data(&self, event: &PlatformEventData) -> dioxus_html::TouchData {
        self.inner.convert_touch_data(event)
    }

    fn convert_transition_data(&self, event: &PlatformEventData) -> dioxus_html::TransitionData {
        self.inner.convert_transition_data(event)
    }

    fn convert_visible_data(&self, event: &PlatformEventData) -> dioxus_html::VisibleData {
        self.inner.convert_visible_data(event)
    }

    fn convert_wheel_data(&self, event: &PlatformEventData) -> dioxus_html::WheelData {
        self.inner.convert_wheel_data(event)
    }
}
