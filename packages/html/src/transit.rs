use std::{any::Any, rc::Rc};

use crate::events::*;
use dioxus_core::ElementId;
use serde::{Deserialize, Serialize};

#[cfg(feature = "serialize")]
#[derive(Serialize, Debug, PartialEq)]
pub struct HtmlEvent {
    pub element: ElementId,
    pub name: String,
    pub bubbles: bool,
    pub data: EventData,
}

#[cfg(feature = "serialize")]
impl<'de> Deserialize<'de> for HtmlEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug, Clone)]
        struct Inner {
            element: ElementId,
            name: String,
            bubbles: bool,
            data: serde_json::Value,
        }

        let Inner {
            element,
            name,
            bubbles,
            data,
        } = Inner::deserialize(deserializer)?;

        // in debug mode let's try and be helpful as to why the deserialization failed
        let data = deserialize_raw(&name, &data).map_err(|e| {
            serde::de::Error::custom(format!(
                "Failed to deserialize event data for event {}:  {}\n'{:#?}'",
                name, e, data,
            ))
        })?;

        Ok(HtmlEvent {
            data,
            element,
            bubbles,
            name,
        })
    }
}

#[cfg(feature = "serialize")]
fn deserialize_raw(name: &str, data: &serde_json::Value) -> Result<EventData, serde_json::Error> {
    match deserialize_raw_event(name, data)? {
        Some(result) => Ok(result),
        None => Err(serde::de::Error::custom(format!(
            "Unknown event type: {name}"
        ))),
    }
}

#[cfg(feature = "serialize")]
impl HtmlEvent {
    pub fn bubbles(&self) -> bool {
        self.bubbles
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EventData {
    Cancel(SerializedCancelData),
    Mouse(SerializedMouseData),
    Clipboard(SerializedClipboardData),
    Composition(SerializedCompositionData),
    Keyboard(SerializedKeyboardData),
    Focus(SerializedFocusData),
    Form(SerializedFormData),
    Drag(SerializedDragData),
    Pointer(SerializedPointerData),
    Selection(SerializedSelectionData),
    Touch(SerializedTouchData),
    Resize(SerializedResizeData),
    Scroll(SerializedScrollData),
    Visible(SerializedVisibleData),
    Wheel(SerializedWheelData),
    Media(SerializedMediaData),
    Animation(SerializedAnimationData),
    Transition(SerializedTransitionData),
    Toggle(SerializedToggleData),
    Image(SerializedImageData),
    Mounted,
}

impl EventData {
    pub fn into_any(self) -> Rc<dyn Any> {
        match self {
            EventData::Cancel(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Mouse(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Clipboard(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Composition(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Keyboard(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Focus(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Form(data) => Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            EventData::Drag(data) => Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            EventData::Pointer(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Selection(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Touch(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Resize(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Scroll(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Visible(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Wheel(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Media(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Animation(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Transition(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Toggle(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Image(data) => {
                Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>
            }
            EventData::Mounted => Rc::new(PlatformEventData::new(Box::new(()))) as Rc<dyn Any>,
        }
    }
}

pub struct SerializedHtmlEventConverter;

impl HtmlEventConverter for SerializedHtmlEventConverter {
    fn convert_animation_data(&self, event: &PlatformEventData) -> AnimationData {
        event
            .downcast::<SerializedAnimationData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_cancel_data(&self, event: &PlatformEventData) -> CancelData {
        event
            .downcast::<SerializedCancelData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_clipboard_data(&self, event: &PlatformEventData) -> ClipboardData {
        event
            .downcast::<SerializedClipboardData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_composition_data(&self, event: &PlatformEventData) -> CompositionData {
        event
            .downcast::<SerializedCompositionData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_drag_data(&self, event: &PlatformEventData) -> DragData {
        event
            .downcast::<SerializedDragData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_focus_data(&self, event: &PlatformEventData) -> FocusData {
        event
            .downcast::<SerializedFocusData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        event
            .downcast::<SerializedFormData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_image_data(&self, event: &PlatformEventData) -> ImageData {
        event
            .downcast::<SerializedImageData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        event
            .downcast::<SerializedKeyboardData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_media_data(&self, event: &PlatformEventData) -> MediaData {
        event
            .downcast::<SerializedMediaData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_mounted_data(&self, _: &PlatformEventData) -> MountedData {
        MountedData::new(())
    }
    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<SerializedMouseData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData {
        event
            .downcast::<SerializedPointerData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_resize_data(&self, event: &PlatformEventData) -> ResizeData {
        event
            .downcast::<SerializedResizeData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_scroll_data(&self, event: &PlatformEventData) -> ScrollData {
        event
            .downcast::<SerializedScrollData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_selection_data(&self, event: &PlatformEventData) -> SelectionData {
        event
            .downcast::<SerializedSelectionData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_toggle_data(&self, event: &PlatformEventData) -> ToggleData {
        event
            .downcast::<SerializedToggleData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_touch_data(&self, event: &PlatformEventData) -> TouchData {
        event
            .downcast::<SerializedTouchData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_transition_data(&self, event: &PlatformEventData) -> TransitionData {
        event
            .downcast::<SerializedTransitionData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_visible_data(&self, event: &PlatformEventData) -> VisibleData {
        event
            .downcast::<SerializedVisibleData>()
            .unwrap()
            .clone()
            .into()
    }
    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData {
        event
            .downcast::<SerializedWheelData>()
            .unwrap()
            .clone()
            .into()
    }
}
