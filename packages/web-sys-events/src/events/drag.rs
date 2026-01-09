use crate::{WebDataTransfer, WebFileData, WebFileEngine};

use super::{Synthetic, WebEventExt};
use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{decode_mouse_button_set, MouseButton},
    FileData, HasDataTransferData, HasDragData, HasFileData, HasMouseData,
    InteractionElementOffset, InteractionLocation, Modifiers, ModifiersInteraction,
    PointerInteraction,
};
use web_sys::{DragEvent, FileReader};

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

impl HasDataTransferData for Synthetic<DragEvent> {
    fn data_transfer(&self) -> dioxus_html::DataTransfer {
        use wasm_bindgen::JsCast;

        if let Some(target) = self.event.dyn_ref::<web_sys::DragEvent>() {
            if let Some(data) = target.data_transfer() {
                let web_data_transfer = WebDataTransfer::new(data);
                return dioxus_html::DataTransfer::new(web_data_transfer);
            }
        }

        // Return an empty DataTransfer if we couldn't get one from the event
        let web_data_transfer = WebDataTransfer::new(web_sys::DataTransfer::new().unwrap());
        dioxus_html::DataTransfer::new(web_data_transfer)
    }
}

impl HasFileData for Synthetic<DragEvent> {
    fn files(&self) -> Vec<FileData> {
        use wasm_bindgen::JsCast;

        if let Some(target) = self.event.dyn_ref::<web_sys::DragEvent>() {
            if let Some(data_transfer) = target.data_transfer() {
                if let Some(file_list) = data_transfer.files() {
                    return WebFileEngine::new(file_list).to_files();
                } else {
                    let items = data_transfer.items();
                    let mut files = vec![];
                    for i in 0..items.length() {
                        if let Some(item) = items.get(i) {
                            if item.kind() == "file" {
                                if let Ok(Some(file)) = item.get_as_file() {
                                    let web_data =
                                        WebFileData::new(file, FileReader::new().unwrap());
                                    files.push(FileData::new(web_data));
                                }
                            }
                        }
                    }
                    return files;
                }
            }
        } else {
            tracing::warn!("DragEvent target was not a DragEvent");
        }

        vec![]
    }
}

impl WebEventExt for dioxus_html::DragData {
    type WebEvent = web_sys::DragEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::DragEvent> {
        self.downcast::<DragEvent>().cloned()
    }
}
