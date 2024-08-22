use std::{any::Any, collections::HashMap};

use dioxus_html::{
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    AnimationData, DragData, FormData, FormValue, HasDragData, HasFileData, HasFormData,
    HasImageData, HasMouseData, HtmlEventConverter, ImageData, MountedData, PlatformEventData,
    ScrollData,
};
use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::{Document, Element, Event, MouseEvent};

#[derive(Clone)]
pub struct WebImageEvent {
    raw: Event,
    error: bool,
}

impl WebImageEvent {
    pub fn new(raw: Event, error: bool) -> Self {
        Self { raw, error }
    }
}

impl HasImageData for WebImageEvent {
    fn load_error(&self) -> bool {
        self.error
    }

    fn as_any(&self) -> &dyn Any {
        &self.raw as &dyn Any
    }
}
