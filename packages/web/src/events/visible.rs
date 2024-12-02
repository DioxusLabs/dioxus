use crate::events::HasKeyboardData;
use crate::events::{
    AnimationData, CompositionData, KeyboardData, MouseData, PointerData, TouchData,
    TransitionData, WheelData,
};
use crate::file_data::HasFileData;
use crate::geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint};
use crate::geometry::{PixelsRect, PixelsSize};
use crate::input_data::{decode_key_location, decode_mouse_button_set, MouseButton};
use crate::prelude::*;

use dioxus_core::ElementId;
use euclid::{Point2D, Size2D};
use keyboard_types::{Code, Key, Modifiers};
use std::str::FromStr;
use wasm_bindgen::JsCast;
use web_sys::{js_sys, DomRectReadOnly, IntersectionObserverEntry, ResizeObserverEntry};
use web_sys::{
    AnimationEvent, CompositionEvent, CustomEvent, Event, KeyboardEvent, MouseEvent, PointerEvent,
    Touch, TouchEvent, TransitionEvent, WheelEvent,
};

impl From<Event> for VisibleData {
    #[inline]
    fn from(e: Event) -> Self {
        <VisibleData as From<&Event>>::from(&e)
    }
}

impl From<&Event> for VisibleData {
    #[inline]
    fn from(e: &Event) -> Self {
        let e: &CustomEvent = e.unchecked_ref();
        let value = e.detail();
        Self::from(value.unchecked_into::<IntersectionObserverEntry>())
    }
}

impl WebEventExt<web_sys::IntersectionObserverEntry> for dioxus_html::VisibleData {
    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::IntersectionObserverEntry> {
        self.downcast::<web_sys::CustomEvent>().and_then(|e| {
            e.detail()
                .dyn_into::<web_sys::IntersectionObserverEntry>()
                .ok()
        })
    }
}
