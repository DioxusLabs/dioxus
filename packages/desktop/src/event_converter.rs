//! Desktop-specific event converter that wraps WebEventConverter.
//!
//! This module provides a custom event converter for desktop that handles
//! desktop-specific event types like DesktopFileDragEvent which include
//! native file paths.

use crate::file_upload::{DesktopFileData, DesktopFileDragEvent, DesktopFormData};
use dioxus_html::{DragData, FileData, FormData, FormValue, HtmlEventConverter, PlatformEventData};
use dioxus_web_sys_events::{GenericWebSysEvent, WebEventConverter};
use std::path::PathBuf;
use web_sys::wasm_bindgen::JsCast;

/// Implement `HtmlEventConverter` methods by forwarding to `self.inner`.
macro_rules! delegate_to_inner {
    ($($method:ident => $ret:ty),* $(,)?) => {
        $(
            fn $method(&self, event: &PlatformEventData) -> $ret {
                self.inner.$method(event)
            }
        )*
    };
}

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
    delegate_to_inner! {
        convert_animation_data => dioxus_html::AnimationData,
        convert_before_input_data => dioxus_html::BeforeInputData,
        convert_cancel_data => dioxus_html::CancelData,
        convert_clipboard_data => dioxus_html::ClipboardData,
        convert_composition_data => dioxus_html::CompositionData,
        convert_focus_data => dioxus_html::FocusData,
        convert_image_data => dioxus_html::ImageData,
        convert_keyboard_data => dioxus_html::KeyboardData,
        convert_media_data => dioxus_html::MediaData,
        convert_mounted_data => dioxus_html::MountedData,
        convert_mouse_data => dioxus_html::MouseData,
        convert_pointer_data => dioxus_html::PointerData,
        convert_resize_data => dioxus_html::ResizeData,
        convert_scroll_data => dioxus_html::ScrollData,
        convert_selection_data => dioxus_html::SelectionData,
        convert_toggle_data => dioxus_html::ToggleData,
        convert_touch_data => dioxus_html::TouchData,
        convert_transition_data => dioxus_html::TransitionData,
        convert_visible_data => dioxus_html::VisibleData,
        convert_wheel_data => dioxus_html::WheelData,
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> dioxus_html::FormData {
        // Check for web-sys events (from wry-bindgen bridge)
        if let Some(web_event) = event.downcast::<GenericWebSysEvent>() {
            // Check if this is a file input
            if let Some(input) = web_event.element.dyn_ref::<web_sys::HtmlInputElement>() {
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
                                } else {
                                    tracing::warn!(
                                        "skipping file input entry whose name is not an existing \
                                         native path: {path:?}"
                                    );
                                }
                            }
                        }

                        if !values.is_empty() {
                            let values = merge_desktop_file_values(
                                self.inner.convert_form_data(event).values(),
                                input_name.as_str(),
                                values,
                            );

                            return FormData::new(DesktopFormData::new(input.value(), values));
                        }
                    }
                }
            }
        }

        // Fall back to web-sys conversion
        self.inner.convert_form_data(event)
    }
}

fn merge_desktop_file_values(
    web_values: Vec<(String, FormValue)>,
    input_name: &str,
    desktop_file_values: Vec<(String, FormValue)>,
) -> Vec<(String, FormValue)> {
    let mut merged = Vec::with_capacity(web_values.len() + desktop_file_values.len());
    let mut desktop_file_values = Some(desktop_file_values);

    for (key, value) in web_values {
        if key == input_name && matches!(value, FormValue::File(_)) {
            if let Some(desktop_file_values) = desktop_file_values.take() {
                merged.extend(desktop_file_values);
            }
        } else {
            merged.push((key, value));
        }
    }

    if let Some(desktop_file_values) = desktop_file_values {
        merged.extend(desktop_file_values);
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_values_replace_only_the_file_input_entries() {
        let web_values = vec![
            ("username".to_string(), FormValue::Text("ada".to_string())),
            ("avatar".to_string(), FormValue::File(None)),
            ("color".to_string(), FormValue::Text("red".to_string())),
        ];
        let desktop_file_values = vec![(
            "avatar".to_string(),
            FormValue::File(Some(FileData::new(DesktopFileData(PathBuf::from(
                "/tmp/avatar.png",
            ))))),
        )];

        let merged = merge_desktop_file_values(web_values, "avatar", desktop_file_values);

        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged[0],
            ("username".to_string(), FormValue::Text("ada".to_string()))
        );
        assert!(matches!(merged[1].1, FormValue::File(Some(_))));
        assert_eq!(merged[1].0, "avatar");
        assert_eq!(
            merged[2],
            ("color".to_string(), FormValue::Text("red".to_string()))
        );
    }

    #[test]
    fn desktop_file_values_are_appended_when_web_values_have_no_file_entry() {
        let web_values = vec![("username".to_string(), FormValue::Text("ada".to_string()))];
        let desktop_file_values = vec![("upload".to_string(), FormValue::File(None))];

        let merged = merge_desktop_file_values(web_values, "upload", desktop_file_values);

        assert_eq!(
            merged,
            vec![
                ("username".to_string(), FormValue::Text("ada".to_string())),
                ("upload".to_string(), FormValue::File(None)),
            ]
        );
    }
}
