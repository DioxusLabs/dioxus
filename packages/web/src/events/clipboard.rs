use dioxus_html::{DataTransfer, FileData, HasClipboardData, HasDataTransferData, HasFileData};
use web_sys::ClipboardEvent;

use crate::WebDataTransfer;

use super::WebEventExt;

pub(crate) struct WebClipboardData {
    event: ClipboardEvent,
}

impl WebClipboardData {
    pub fn new(event: ClipboardEvent) -> Self {
        Self { event }
    }
}

impl HasClipboardData for WebClipboardData {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event as &dyn std::any::Any
    }
}

impl HasDataTransferData for WebClipboardData {
    fn data_transfer(&self) -> DataTransfer {
        let data = self
            .event
            .clipboard_data()
            // No `clipboardData` (e.g. `copy`/`cut` until set): fall back to an empty transfer.
            .unwrap_or_else(|| web_sys::DataTransfer::new().unwrap());
        DataTransfer::new(WebDataTransfer::new(data))
    }
}

impl HasFileData for WebClipboardData {
    fn files(&self) -> Vec<FileData> {
        self.data_transfer().files()
    }
}

impl WebEventExt for dioxus_html::ClipboardData {
    type WebEvent = web_sys::ClipboardEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::ClipboardEvent>().cloned()
    }
}
