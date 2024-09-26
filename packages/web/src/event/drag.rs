use dioxus_html::{
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    HasDragData, HasFileData, HasMouseData,
};
use wasm_bindgen::JsCast;
use web_sys::MouseEvent;

use super::synthetic::Synthetic;

pub struct WebDragData {
    raw: Synthetic<MouseEvent>,
}

impl WebDragData {
    pub fn new(raw: MouseEvent) -> Self {
        Self {
            raw: Synthetic::new(raw),
        }
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
        self.raw.trigger_button()
    }

    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        self.raw.held_buttons()
    }
}

impl ModifiersInteraction for WebDragData {
    fn modifiers(&self) -> dioxus_html::prelude::Modifiers {
        self.raw.modifiers()
    }
}

impl InteractionElementOffset for WebDragData {
    fn coordinates(&self) -> dioxus_html::geometry::Coordinates {
        self.raw.coordinates()
    }

    fn element_coordinates(&self) -> dioxus_html::geometry::ElementPoint {
        self.raw.element_coordinates()
    }
}

impl InteractionLocation for WebDragData {
    fn client_coordinates(&self) -> dioxus_html::geometry::ClientPoint {
        self.raw.client_coordinates()
    }

    fn screen_coordinates(&self) -> dioxus_html::geometry::ScreenPoint {
        self.raw.screen_coordinates()
    }

    fn page_coordinates(&self) -> dioxus_html::geometry::PagePoint {
        self.raw.page_coordinates()
    }
}

impl HasFileData for WebDragData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        use super::file::WebFileEngine;

        self.raw
            .event
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
            })
    }
}
