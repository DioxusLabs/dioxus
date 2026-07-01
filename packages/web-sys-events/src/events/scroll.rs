use dioxus_html::HasScrollData;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event};

use super::{Synthetic, WebEventExt};

impl HasScrollData for Synthetic<Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }

    fn scroll_top(&self) -> f64 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.scroll_top() as f64;
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.scroll_top() as f64;
            }
        }
        0f64
    }

    fn scroll_left(&self) -> f64 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.scroll_left() as f64;
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.scroll_left() as f64;
            }
        }
        0f64
    }

    fn scroll_width(&self) -> i32 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.scroll_width();
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.scroll_width();
            }
        }
        0
    }

    fn scroll_height(&self) -> i32 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.scroll_height();
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.scroll_height();
            }
        }
        0
    }

    fn client_width(&self) -> i32 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.client_width();
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.client_width();
            }
        }
        0
    }

    fn client_height(&self) -> i32 {
        if let Some(target) = self.event.target().as_ref() {
            if let Some(element) = target.dyn_ref::<Element>() {
                return element.client_height();
            } else if let Some(element) = target
                .dyn_ref::<Document>()
                .and_then(|document| document.document_element())
            {
                return element.client_height();
            }
        }
        0
    }
}

impl WebEventExt for dioxus_html::ScrollData {
    type WebEvent = web_sys::Event;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::Event>().cloned()
    }
}
