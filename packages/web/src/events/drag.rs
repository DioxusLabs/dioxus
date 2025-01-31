use super::{Synthetic, WebEventExt};
use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{decode_mouse_button_set, MouseButton},
    prelude::{
        InteractionElementOffset, InteractionLocation, Modifiers, ModifiersInteraction,
        PointerInteraction,
    },
    HasDragData, HasFileData, HasMouseData,
};
use web_sys::DragEvent;

impl InteractionLocation for Synthetic<DragEvent> {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.event.client_x().into(), self.event.client_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.event.page_x().into(), self.event.page_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.event.screen_x().into(), self.event.screen_y().into())
    }
}

impl InteractionElementOffset for Synthetic<DragEvent> {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.event.offset_x().into(), self.event.offset_y().into())
    }
}

impl ModifiersInteraction for Synthetic<DragEvent> {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.event.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.event.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.event.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.event.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl PointerInteraction for Synthetic<DragEvent> {
    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl HasMouseData for Synthetic<DragEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl HasDragData for Synthetic<DragEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl HasFileData for Synthetic<DragEvent> {
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        #[cfg(feature = "file_engine")]
        {
            use wasm_bindgen::JsCast;
            let files = self
                .event
                .dyn_ref::<web_sys::DragEvent>()
                .and_then(|drag_event| {
                    drag_event.data_transfer().and_then(|dt| {
                        dt.files().and_then(|files| {
                            #[allow(clippy::arc_with_non_send_sync)]
                            crate::file_engine::WebFileEngine::new(files).map(|f| {
                                std::sync::Arc::new(f)
                                    as std::sync::Arc<dyn dioxus_html::FileEngine>
                            })
                        })
                    })
                });

            files
        }
        #[cfg(not(feature = "file_engine"))]
        {
            None
        }
    }
}

impl WebEventExt for dioxus_html::DragData {
    type WebEvent = web_sys::DragEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::DragEvent> {
        self.downcast::<DragEvent>().cloned()
    }
}
