use std::time::SystemTime;

use dioxus_html::{
    geometry::{
        euclid::{Point2D, Size2D},
        PixelsRect,
    },
    HasVisibleData, VisibleData, VisibleError, VisibleResult,
};
use wasm_bindgen::JsCast;
use web_sys::{CustomEvent, DomRectReadOnly, Event, IntersectionObserverEntry};

use super::{Synthetic, WebEventExt};

impl From<Event> for Synthetic<IntersectionObserverEntry> {
    #[inline]
    fn from(e: Event) -> Self {
        <Synthetic<IntersectionObserverEntry> as From<&Event>>::from(&e)
    }
}

impl From<&Event> for Synthetic<IntersectionObserverEntry> {
    #[inline]
    fn from(e: &Event) -> Self {
        let e: &CustomEvent = e.unchecked_ref();
        let value = e.detail();
        Self::new(value.unchecked_into::<IntersectionObserverEntry>())
    }
}
fn dom_rect_ro_to_pixel_rect(dom_rect: &DomRectReadOnly) -> PixelsRect {
    PixelsRect::new(
        Point2D::new(dom_rect.x(), dom_rect.y()),
        Size2D::new(dom_rect.width(), dom_rect.height()),
    )
}

impl HasVisibleData for Synthetic<IntersectionObserverEntry> {
    /// Get the bounds rectangle of the target element
    fn get_bounding_client_rect(&self) -> VisibleResult<PixelsRect> {
        Ok(dom_rect_ro_to_pixel_rect(
            &self.event.bounding_client_rect(),
        ))
    }

    /// Get the ratio of the intersectionRect to the boundingClientRect
    fn get_intersection_ratio(&self) -> VisibleResult<f64> {
        Ok(self.event.intersection_ratio())
    }

    /// Get the rect representing the target's visible area
    fn get_intersection_rect(&self) -> VisibleResult<PixelsRect> {
        Ok(dom_rect_ro_to_pixel_rect(&self.event.intersection_rect()))
    }

    /// Get if the target element intersects with the intersection observer's root
    fn is_intersecting(&self) -> VisibleResult<bool> {
        Ok(self.event.is_intersecting())
    }

    /// Get the rect for the intersection observer's root
    fn get_root_bounds(&self) -> VisibleResult<PixelsRect> {
        match self.event.root_bounds() {
            Some(root_bounds) => Ok(dom_rect_ro_to_pixel_rect(&root_bounds)),
            None => Err(VisibleError::NotSupported),
        }
    }

    /// Get a timestamp indicating the time at which the intersection was recorded
    fn get_time(&self) -> VisibleResult<SystemTime> {
        let ms_since_epoch = self.event.time();
        let rounded = ms_since_epoch.round() as u64;
        let duration = std::time::Duration::from_millis(rounded);
        Ok(SystemTime::UNIX_EPOCH + duration)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WebEventExt for VisibleData {
    type WebEvent = IntersectionObserverEntry;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<IntersectionObserverEntry> {
        self.downcast::<CustomEvent>()
            .and_then(|e| e.detail().dyn_into::<IntersectionObserverEntry>().ok())
    }
}
