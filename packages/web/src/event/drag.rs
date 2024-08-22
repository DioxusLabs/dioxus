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

pub struct WebDragData {
    raw: MouseEvent,
}

impl WebDragData {
    pub fn new(raw: MouseEvent) -> Self {
        Self { raw }
    }
}

impl HasDragData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.raw as &dyn std::any::Any
    }
}

impl HasMouseData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.raw as &dyn std::any::Any
    }
}

impl PointerInteraction for WebDragData {
    fn trigger_button(&self) -> Option<dioxus_html::input_data::MouseButton> {
        // self.raw.trigger_button()
        todo!()
    }

    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        // self.raw.held_buttons()
        todo!()
    }
}

impl ModifiersInteraction for WebDragData {
    fn modifiers(&self) -> dioxus_html::prelude::Modifiers {
        // self.raw.modifiers()
        todo!()
    }
}

impl InteractionElementOffset for WebDragData {
    fn coordinates(&self) -> dioxus_html::geometry::Coordinates {
        // self.raw.coordinates()
        todo!()
    }

    fn element_coordinates(&self) -> dioxus_html::geometry::ElementPoint {
        // self.raw.element_coordinates()
        todo!()
    }
}

impl InteractionLocation for WebDragData {
    fn client_coordinates(&self) -> dioxus_html::geometry::ClientPoint {
        // self.raw.client_coordinates()
        todo!()
    }

    fn screen_coordinates(&self) -> dioxus_html::geometry::ScreenPoint {
        // self.raw.screen_coordinates()
        todo!()
    }

    fn page_coordinates(&self) -> dioxus_html::geometry::PagePoint {
        // self.raw.page_coordinates()
        todo!()
    }
}

impl HasFileData for WebDragData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        use crate::bindings::WebFileEngine;

        let files = self
            .raw
            .dyn_ref::<web_sys::DragEvent>()
            .and_then(|drag_event| {
                drag_event.data_transfer().and_then(|dt| {
                    dt.files().and_then(|files| {
                        #[allow(clippy::arc_with_non_send_sync)]
                        WebFileEngine::new(files).map(|f| {
                            std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                        })
                    })
                })
            });

        files
    }
}
