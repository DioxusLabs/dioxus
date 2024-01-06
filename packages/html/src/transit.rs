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
            data: serde_value::Value,
        }

        let Inner {
            element,
            name,
            bubbles,
            data,
        } = Inner::deserialize(deserializer)?;

        Ok(HtmlEvent {
            data: fun_name(&name, data).unwrap(),
            element,
            bubbles,
            name,
        })
    }
}

#[cfg(feature = "serialize")]
fn fun_name(
    name: &str,
    data: serde_value::Value,
) -> Result<EventData, serde_value::DeserializerError> {
    use EventData::*;

    // a little macro-esque thing to make the code below more readable
    #[inline]
    fn de<'de, F>(f: serde_value::Value) -> Result<F, serde_value::DeserializerError>
    where
        F: Deserialize<'de>,
    {
        F::deserialize(f)
    }

    let data = match name {
        // Mouse
        "click" | "contextmenu" | "dblclick" | "doubleclick" | "mousedown" | "mouseenter"
        | "mouseleave" | "mousemove" | "mouseout" | "mouseover" | "mouseup" => Mouse(de(data)?),

        // Clipboard
        "copy" | "cut" | "paste" => Clipboard(de(data)?),

        // Composition
        "compositionend" | "compositionstart" | "compositionupdate" => Composition(de(data)?),

        // Keyboard
        "keydown" | "keypress" | "keyup" => Keyboard(de(data)?),

        // Focus
        "blur" | "focus" | "focusin" | "focusout" => Focus(de(data)?),

        // Form
        "change" | "input" | "invalid" | "reset" | "submit" => Form(de(data)?),

        // Drag
        "drag" | "dragend" | "dragenter" | "dragexit" | "dragleave" | "dragover" | "dragstart"
        | "drop" => Drag(de(data)?),

        // Pointer
        "pointerlockchange" | "pointerlockerror" | "pointerdown" | "pointermove" | "pointerup"
        | "pointerover" | "pointerout" | "pointerenter" | "pointerleave" | "gotpointercapture"
        | "lostpointercapture" => Pointer(de(data)?),

        // Selection
        "selectstart" | "selectionchange" | "select" => Selection(de(data)?),

        // Touch
        "touchcancel" | "touchend" | "touchmove" | "touchstart" => Touch(de(data)?),

        // Scroll
        "scroll" => Scroll(de(data)?),

        // Wheel
        "wheel" => Wheel(de(data)?),

        // Media
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "interruptbegin" | "interruptend" | "loadeddata" | "loadedmetadata"
        | "loadstart" | "pause" | "play" | "playing" | "progress" | "ratechange" | "seeked"
        | "seeking" | "stalled" | "suspend" | "timeupdate" | "volumechange" | "waiting"
        | "loadend" | "timeout" => Media(de(data)?),

        // Animation
        "animationstart" | "animationend" | "animationiteration" => Animation(de(data)?),

        // Transition
        "transitionend" => Transition(de(data)?),

        // Toggle
        "toggle" => Toggle(de(data)?),

        "load" | "error" => Image(de(data)?),

        // Mounted
        "mounted" => Mounted,

        // OtherData => "abort" | "afterprint" | "beforeprint" | "beforeunload" | "hashchange" | "languagechange" | "message" | "offline" | "online" | "pagehide" | "pageshow" | "popstate" | "rejectionhandled" | "storage" | "unhandledrejection" | "unload" | "userproximity" | "vrdisplayactivate" | "vrdisplayblur" | "vrdisplayconnect" | "vrdisplaydeactivate" | "vrdisplaydisconnect" | "vrdisplayfocus" | "vrdisplaypointerrestricted" | "vrdisplaypointerunrestricted" | "vrdisplaypresentchange";
        other => {
            return Err(serde_value::DeserializerError::UnknownVariant(
                other.to_string(),
                &[],
            ))
        }
    };

    Ok(data)
}

#[cfg(feature = "serialize")]
impl HtmlEvent {
    pub fn bubbles(&self) -> bool {
        event_bubbles(&self.name)
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EventData {
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
    Scroll(SerializedScrollData),
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
            EventData::Scroll(data) => {
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
            EventData::Mounted => {
                Rc::new(PlatformEventData::new(Box::new(MountedData::new(())))) as Rc<dyn Any>
            }
        }
    }
}

#[test]
fn test_back_and_forth() {
    let data = HtmlEvent {
        element: ElementId(0),
        data: EventData::Mouse(SerializedMouseData::default()),
        name: "click".to_string(),
        bubbles: true,
    };

    println!("{}", serde_json::to_string_pretty(&data).unwrap());

    let o = r#"
{
  "element": 0,
  "name": "click",
  "bubbles": true,
  "data": {
    "alt_key": false,
    "button": 0,
    "buttons": 0,
    "client_x": 0,
    "client_y": 0,
    "ctrl_key": false,
    "meta_key": false,
    "offset_x": 0,
    "offset_y": 0,
    "page_x": 0,
    "page_y": 0,
    "screen_x": 0,
    "screen_y": 0,
    "shift_key": false
  }
}
    "#;

    let p: HtmlEvent = serde_json::from_str(o).unwrap();

    assert_eq!(data, p);
}

/// A trait for converting from a serialized event to a concrete event type.
pub struct SerializedHtmlEventConverter;

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
        event
            .downcast::<SerializedDragData>()
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
        event
            .downcast::<SerializedFormData>()
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

    fn convert_mounted_data(&self, _: &PlatformEventData) -> MountedData {
        MountedData::from(())
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
