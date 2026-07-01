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
        let form_data = self.inner.convert_form_data(event);
        let mut values = form_data.values();
        let mut has_desktop_file_values = false;

        if let Some(web_event) = event.downcast::<GenericWebSysEvent>() {
            for input in file_inputs_for_event(&web_event.element) {
                let desktop_file_values = desktop_file_values_from_input(&input);

                if !desktop_file_values.is_empty() {
                    replace_file_values(&mut values, input.name().as_str(), desktop_file_values);
                    has_desktop_file_values = true;
                }
            }
        }

        if has_desktop_file_values {
            FormData::new(DesktopFormData::new(form_data.value(), values))
        } else {
            form_data
        }
    }
}

fn file_inputs_for_event(element: &web_sys::Element) -> Vec<web_sys::HtmlInputElement> {
    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        return (input.type_() == "file")
            .then(|| input.clone())
            .into_iter()
            .collect();
    }

    let Ok(Some(form)) = element.closest("form") else {
        return Vec::new();
    };

    let Ok(inputs) = form.query_selector_all("input[type='file']") else {
        return Vec::new();
    };

    let mut file_inputs = Vec::new();
    for index in 0..inputs.length() {
        if let Some(input) = inputs
            .item(index)
            .and_then(|node| node.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            file_inputs.push(input);
        }
    }

    file_inputs
}

fn desktop_file_values_from_input(input: &web_sys::HtmlInputElement) -> Vec<(String, FormValue)> {
    let mut values = Vec::new();
    let input_name = input.name();

    let Some(file_list) = input.files() else {
        return values;
    };

    for i in 0..file_list.length() {
        if let Some(file) = file_list.get(i) {
            // The filename is actually the native path.
            let path = PathBuf::from(file.name());
            if let Some(file_data) = desktop_file_data_from_path(path.clone()) {
                values.push((input_name.clone(), FormValue::File(Some(file_data))));
            } else {
                tracing::warn!(
                    "skipping file input entry whose name is not an existing native path: {path:?}"
                );
            }
        }
    }

    values
}

fn desktop_file_data_from_path(path: PathBuf) -> Option<FileData> {
    path.exists().then(|| FileData::new(DesktopFileData(path)))
}

fn replace_file_values(
    values: &mut Vec<(String, FormValue)>,
    input_name: &str,
    desktop_file_values: Vec<(String, FormValue)>,
) {
    let mut desktop_file_values = desktop_file_values.into_iter();

    for (key, value) in values.iter_mut() {
        if key == input_name && matches!(value, FormValue::File(_)) {
            if let Some((_, desktop_file_value)) = desktop_file_values.next() {
                *value = desktop_file_value;
            }
        }
    }

    values.extend(desktop_file_values);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_file_values_replace_empty_file_placeholders() {
        let avatar_path = test_file_path("avatar", b"avatar");

        let mut values = vec![
            ("username".to_string(), FormValue::Text("ada".to_string())),
            ("avatar".to_string(), FormValue::File(None)),
            ("color".to_string(), FormValue::Text("red".to_string())),
        ];
        let desktop_file_values = vec![(
            "avatar".to_string(),
            FormValue::File(Some(FileData::new(DesktopFileData(avatar_path.clone())))),
        )];

        replace_file_values(&mut values, "avatar", desktop_file_values);

        assert_eq!(values.len(), 3);
        assert_eq!(
            values[0],
            ("username".to_string(), FormValue::Text("ada".to_string()))
        );
        assert_file_value(&values[1], "avatar", avatar_path.as_path(), 6);
        assert_eq!(
            values[2],
            ("color".to_string(), FormValue::Text("red".to_string()))
        );
    }

    fn test_file_path(name: &str, contents: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "dioxus-desktop-event-converter-{name}-{}",
            std::process::id()
        ));
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn assert_file_value(
        value: &(String, FormValue),
        expected_name: &str,
        expected_path: &std::path::Path,
        expected_size: u64,
    ) {
        assert_eq!(value.0, expected_name);

        let FormValue::File(Some(file)) = &value.1 else {
            panic!("expected file value");
        };

        assert_eq!(file.path(), expected_path);
        assert_eq!(file.size(), expected_size);
    }
}
