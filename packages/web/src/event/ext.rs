/// A extension trait for web-sys events that provides a way to get the event as a web-sys event.
pub trait WebEventExt<E> {
    /// Try to downcast this event as a `web-sys` event.
    fn try_as_web_event(&self) -> Option<E>;

    /// Downcast this event as a `web-sys` event.
    #[inline(always)]
    fn as_web_event(&self) -> E
    where
        E: 'static,
    {
        self.try_as_web_event().unwrap_or_else(|| {
            panic!(
                "Error downcasting to `web-sys`, event should be a {}.",
                std::any::type_name::<E>()
            )
        })
    }
}

// impl WebEventExt<web_sys::AnimationEvent> for dioxus_html::AnimationData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::AnimationEvent> {
//         self.downcast::<web_sys::AnimationEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for dioxus_html::ClipboardData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<web_sys::CompositionEvent> for dioxus_html::CompositionData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::CompositionEvent> {
//         self.downcast::<web_sys::CompositionEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::MouseEvent> for dioxus_html::DragData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::MouseEvent> {
//         self.downcast::<WebDragData>()
//             .map(|data| &data.raw)
//             .cloned()
//     }
// }

// impl WebEventExt<web_sys::FocusEvent> for dioxus_html::FocusData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::FocusEvent> {
//         self.downcast::<web_sys::FocusEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for dioxus_html::FormData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<WebImageEvent> for dioxus_html::ImageData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<WebImageEvent> {
//         self.downcast::<WebImageEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::KeyboardEvent> for dioxus_html::KeyboardData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::KeyboardEvent> {
//         self.downcast::<web_sys::KeyboardEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for dioxus_html::MediaData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Element> for MountedData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Element> {
//         self.downcast::<web_sys::Element>().cloned()
//     }
// }

// impl WebEventExt<web_sys::MouseEvent> for dioxus_html::MouseData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::MouseEvent> {
//         self.downcast::<web_sys::MouseEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::PointerEvent> for dioxus_html::PointerData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::PointerEvent> {
//         self.downcast::<web_sys::PointerEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for ScrollData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for dioxus_html::SelectionData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<web_sys::Event> for dioxus_html::ToggleData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::Event> {
//         self.downcast::<web_sys::Event>().cloned()
//     }
// }

// impl WebEventExt<web_sys::TouchEvent> for dioxus_html::TouchData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::TouchEvent> {
//         self.downcast::<web_sys::TouchEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::TransitionEvent> for dioxus_html::TransitionData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::TransitionEvent> {
//         self.downcast::<web_sys::TransitionEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::WheelEvent> for dioxus_html::WheelData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::WheelEvent> {
//         self.downcast::<web_sys::WheelEvent>().cloned()
//     }
// }

// impl WebEventExt<web_sys::ResizeObserverEntry> for dioxus_html::ResizeData {
//     #[inline(always)]
//     fn try_as_web_event(&self) -> Option<web_sys::ResizeObserverEntry> {
//         self.downcast::<web_sys::CustomEvent>()
//             .and_then(|e| e.detail().dyn_into::<web_sys::ResizeObserverEntry>().ok())
//     }
// }
