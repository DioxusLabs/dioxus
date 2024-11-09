use dioxus_html::{geometry::PixelsSize, HasResizeData, ResizeResult};
use wasm_bindgen::JsCast;
use web_sys::{CustomEvent, Event, ResizeObserverEntry};

use super::{Synthetic, WebEventExt};

impl From<Event> for Synthetic<ResizeObserverEntry> {
    #[inline]
    fn from(e: Event) -> Self {
        <Synthetic<ResizeObserverEntry> as From<&Event>>::from(&e)
    }
}

impl From<&Event> for Synthetic<ResizeObserverEntry> {
    #[inline]
    fn from(e: &Event) -> Self {
        let e: &CustomEvent = e.unchecked_ref();
        let value = e.detail();
        Self::new(value.unchecked_into::<ResizeObserverEntry>())
    }
}

impl HasResizeData for Synthetic<ResizeObserverEntry> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }

    fn get_border_box_size(&self) -> ResizeResult<PixelsSize> {
        extract_first_size(self.event.border_box_size())
    }

    fn get_content_box_size(&self) -> ResizeResult<PixelsSize> {
        extract_first_size(self.event.content_box_size())
    }
}

impl WebEventExt for dioxus_html::ResizeData {
    type WebEvent = web_sys::ResizeObserverEntry;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::ResizeObserverEntry> {
        self.downcast::<web_sys::ResizeObserverEntry>().cloned()
    }
}

fn extract_first_size(resize_observer_output: js_sys::Array) -> ResizeResult<PixelsSize> {
    let first = resize_observer_output.get(0);
    let size = first.unchecked_into::<web_sys::ResizeObserverSize>();

    // inline_size matches the width of the element if its writing-mode is horizontal, the height otherwise
    let inline_size = size.inline_size();
    // block_size matches the height of the element if its writing-mode is horizontal, the width otherwise
    let block_size = size.block_size();

    Ok(PixelsSize::new(inline_size, block_size))
}
