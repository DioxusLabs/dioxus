use dioxus_html::{
    prelude::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
    HasDragData, HasFileData, HasMouseData,
};
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, MouseEvent};

use super::{Synthetic, WebEventExt};

pub(crate) struct WebDragData {
    drag: Synthetic<DragEvent>,
    mouse: Synthetic<MouseEvent>,
}

impl WebDragData {
    pub fn new(raw: DragEvent) -> Self {
        let mouse = raw
            .clone()
            .dyn_into::<MouseEvent>()
            .expect("Inconceivable! DragEvent is not MouseEvent?");
        Self {
            drag: Synthetic::new(raw),
            mouse: Synthetic::new(mouse),
        }
    }
}

impl HasDragData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.drag as &dyn std::any::Any
    }
}

impl HasMouseData for WebDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.mouse as &dyn std::any::Any
    }
}

impl PointerInteraction for WebDragData {
    fn trigger_button(&self) -> Option<dioxus_html::input_data::MouseButton> {
        self.mouse.trigger_button()
    }

    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        self.mouse.held_buttons()
    }
}

impl ModifiersInteraction for WebDragData {
    fn modifiers(&self) -> dioxus_html::prelude::Modifiers {
        self.mouse.modifiers()
    }
}

impl InteractionElementOffset for WebDragData {
    fn coordinates(&self) -> dioxus_html::geometry::Coordinates {
        self.mouse.coordinates()
    }

    fn element_coordinates(&self) -> dioxus_html::geometry::ElementPoint {
        self.mouse.element_coordinates()
    }
}

impl InteractionLocation for WebDragData {
    fn client_coordinates(&self) -> dioxus_html::geometry::ClientPoint {
        self.mouse.client_coordinates()
    }

    fn screen_coordinates(&self) -> dioxus_html::geometry::ScreenPoint {
        self.mouse.screen_coordinates()
    }

    fn page_coordinates(&self) -> dioxus_html::geometry::PagePoint {
        self.mouse.page_coordinates()
    }
}

impl HasFileData for WebDragData {
    #[cfg(feature = "file_engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        use wasm_bindgen::JsCast;

        let files = self
            .drag
            .event
            .dyn_ref::<web_sys::DragEvent>()
            .and_then(|drag_event| {
                drag_event.data_transfer().and_then(|dt| {
                    dt.files().and_then(|files| {
                        #[allow(clippy::arc_with_non_send_sync)]
                        crate::file_engine::WebFileEngine::new(files).map(|f| {
                            std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                        })
                    })
                })
            });

        files
    }
}

impl WebEventExt for dioxus_html::DragData {
    type WebEvent = web_sys::DragEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::DragEvent> {
        self.downcast::<WebDragData>()
            .map(|data| &data.drag.event)
            .cloned()
    }
}
