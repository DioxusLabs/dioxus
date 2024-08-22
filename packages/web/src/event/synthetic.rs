use dioxus_html::geometry::PixelsSize;
use dioxus_html::geometry::WheelDelta;
use dioxus_html::geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint};
use dioxus_html::input_data::{decode_key_location, decode_mouse_button_set, MouseButton};
use dioxus_html::prelude::*;
use dioxus_html::HasFileData;
use dioxus_html::{events::HasKeyboardData, input_data::MouseButtonSet};
use keyboard_types::{Code, Key, Modifiers};
use std::str::FromStr;
use wasm_bindgen::JsCast;
use web_sys::{js_sys, ResizeObserverEntry};
use web_sys::{
    AnimationEvent, CompositionEvent, Event, KeyboardEvent, MouseEvent, PointerEvent, Touch,
    TouchEvent, TransitionEvent, WheelEvent,
};

/// A wrapper for the websys event that allows us to give it the impls from dioxus-html
pub struct Synthetic<T: 'static> {
    pub event: T,
}

impl<T: 'static> Synthetic<T> {
    pub fn new(event: T) -> Self {
        Self { event }
    }
}

impl HasCompositionData for Synthetic<CompositionEvent> {
    fn data(&self) -> std::string::String {
        self.event.data().unwrap_or_default()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasKeyboardData for Synthetic<KeyboardEvent> {
    fn key(&self) -> Key {
        Key::from_str(self.event.key().as_str()).unwrap_or(Key::Unidentified)
    }

    fn code(&self) -> Code {
        Code::from_str(self.event.code().as_str()).unwrap_or(Code::Unidentified)
    }

    fn location(&self) -> keyboard_types::Location {
        decode_key_location(self.event.location() as usize)
    }

    fn is_auto_repeating(&self) -> bool {
        self.event.repeat()
    }

    fn is_composing(&self) -> bool {
        self.event.is_composing()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModifiersInteraction for Synthetic<KeyboardEvent> {
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

impl InteractionLocation for Synthetic<MouseEvent> {
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

impl InteractionElementOffset for Synthetic<MouseEvent> {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.event.offset_x().into(), self.event.offset_y().into())
    }
}

impl ModifiersInteraction for Synthetic<MouseEvent> {
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

impl PointerInteraction for Synthetic<MouseEvent> {
    fn held_buttons(&self) -> MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl HasMouseData for Synthetic<MouseEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFileData for Synthetic<MouseEvent> {}

impl HasDragData for Synthetic<MouseEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModifiersInteraction for Synthetic<TouchEvent> {
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

impl HasTouchData for Synthetic<TouchEvent> {
    fn touches(&self) -> Vec<TouchPoint> {
        let touches = TouchEvent::touches(&self.event);
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(Synthetic::new(touch)));
        }
        result
    }

    fn touches_changed(&self) -> Vec<TouchPoint> {
        let touches = self.event.changed_touches();
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(Synthetic::new(touch)));
        }
        result
    }

    fn target_touches(&self) -> Vec<TouchPoint> {
        let touches = self.event.target_touches();
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(Synthetic::new(touch)));
        }
        result
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasTouchPointData for Synthetic<Touch> {
    fn identifier(&self) -> i32 {
        self.event.identifier()
    }

    fn radius(&self) -> ScreenPoint {
        ScreenPoint::new(self.event.radius_x().into(), self.event.radius_y().into())
    }

    fn rotation(&self) -> f64 {
        self.event.rotation_angle() as f64
    }

    fn force(&self) -> f64 {
        self.event.force() as f64
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for Synthetic<Touch> {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.event.client_x().into(), self.event.client_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.event.screen_x().into(), self.event.screen_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.event.page_x().into(), self.event.page_y().into())
    }
}

impl HasPointerData for Synthetic<PointerEvent> {
    fn pointer_id(&self) -> i32 {
        self.event.pointer_id()
    }

    fn width(&self) -> i32 {
        self.event.width()
    }

    fn height(&self) -> i32 {
        self.event.height()
    }

    fn pressure(&self) -> f32 {
        self.event.pressure()
    }

    fn tangential_pressure(&self) -> f32 {
        self.event.tangential_pressure()
    }

    fn tilt_x(&self) -> i32 {
        self.event.tilt_x()
    }

    fn tilt_y(&self) -> i32 {
        self.event.tilt_y()
    }

    fn twist(&self) -> i32 {
        self.event.twist()
    }

    fn pointer_type(&self) -> String {
        self.event.pointer_type()
    }

    fn is_primary(&self) -> bool {
        self.event.is_primary()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for Synthetic<PointerEvent> {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.event.client_x().into(), self.event.client_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.event.screen_x().into(), self.event.screen_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.event.page_x().into(), self.event.page_y().into())
    }
}

impl InteractionElementOffset for Synthetic<PointerEvent> {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.event.offset_x().into(), self.event.offset_y().into())
    }
}

impl ModifiersInteraction for Synthetic<PointerEvent> {
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

impl PointerInteraction for Synthetic<PointerEvent> {
    fn held_buttons(&self) -> MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl HasWheelData for Synthetic<WheelEvent> {
    fn delta(&self) -> WheelDelta {
        WheelDelta::from_web_attributes(
            self.event.delta_mode(),
            self.event.delta_x(),
            self.event.delta_y(),
            self.event.delta_z(),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasMouseData for Synthetic<WheelEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for Synthetic<WheelEvent> {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.event.client_x().into(), self.event.client_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.event.screen_x().into(), self.event.screen_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.event.page_x().into(), self.event.page_y().into())
    }
}

impl InteractionElementOffset for Synthetic<WheelEvent> {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.event.offset_x().into(), self.event.offset_y().into())
    }
}

impl ModifiersInteraction for Synthetic<WheelEvent> {
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

impl PointerInteraction for Synthetic<WheelEvent> {
    fn held_buttons(&self) -> MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl HasAnimationData for Synthetic<AnimationEvent> {
    fn animation_name(&self) -> String {
        self.event.animation_name()
    }

    fn pseudo_element(&self) -> String {
        self.event.pseudo_element()
    }

    fn elapsed_time(&self) -> f32 {
        self.event.elapsed_time()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasTransitionData for Synthetic<TransitionEvent> {
    fn elapsed_time(&self) -> f32 {
        self.event.elapsed_time()
    }

    fn property_name(&self) -> String {
        self.event.property_name()
    }

    fn pseudo_element(&self) -> String {
        self.event.pseudo_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "mounted")]
impl RenderedElementBacking for Synthetic<web_sys::Element> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    // fn get_scroll_offset(
    //     &self,
    // ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<geometry::PixelsVector2D>>>>
    // {
    //     let left = self.event.scroll_left();
    //     let top = self.event.scroll_top();
    //     let result = Ok(geometry::PixelsVector2D::new(left as f64, top as f64));
    //     Box::pin(async { result })
    // }

    // fn get_scroll_size(
    //     &self,
    // ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<geometry::PixelsSize>>>>
    // {
    //     let width = self.event.scroll_width();
    //     let height = self.event.scroll_height();
    //     let result = Ok(geometry::PixelsSize::new(width as f64, height as f64));
    //     Box::pin(async { result })
    // }

    // fn get_client_rect(
    //     &self,
    // ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<geometry::PixelsRect>>>>
    // {
    //     let rect = self.event.get_bounding_client_rect();
    //     let result = Ok(geometry::PixelsRect::new(
    //         euclid::Point2D::new(rect.left(), rect.top()),
    //         euclid::Size2D::new(rect.width(), rect.height()),
    //     ));
    //     Box::pin(async { result })
    // }

    // fn as_any(&self) -> &dyn std::any::Any {
    //     self
    // }

    // fn scroll_to(
    //     &self,
    //     behavior: ScrollBehavior,
    // ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<()>>>> {
    //     let options = web_sys::ScrollIntoViewOptions::new();
    //     match behavior {
    //         ScrollBehavior::Instant => {
    //             options.set_behavior(web_sys::ScrollBehavior::Instant);
    //         }
    //         ScrollBehavior::Smooth => {
    //             options.set_behavior(web_sys::ScrollBehavior::Smooth);
    //         }
    //     }
    //     self.event
    //         .scroll_into_view_with_scroll_into_view_options(&options);

    //     Box::pin(async { Ok(()) })
    // }

    // fn set_focus(
    //     &self,
    //     focus: bool,
    // ) -> std::pin::Pin<Box<dyn std::future::Future<Output = MountedResult<()>>>> {
    //     #[derive(Debug)]
    //     struct FocusError(wasm_bindgen::JsValue);

    //     impl std::fmt::Display for FocusError {
    //         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //             write!(f, "failed to focus element {:?}", self.event.0)
    //         }
    //     }

    //     impl std::error::Error for FocusError {}

    //     let result = self
    //         .dyn_ref::<web_sys::HtmlElement>()
    //         .ok_or_else(|| MountedError::OperationFailed(Box::new(FocusError(self.event.into()))))
    //         .and_then(|e| {
    //             (if focus { e.focus() } else { e.blur() })
    //                 .map_err(|err| MountedError::OperationFailed(Box::new(FocusError(err))))
    //         });
    //     Box::pin(async { result })
    // }
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

// impl From<&Event> for ResizeData {
//     #[inline]
//     fn from(e: &Event) -> Self {
//         let e: &CustomEvent = e.unchecked_ref();
//         let value = e.detail();
//         Self::from(value.unchecked_into::<ResizeObserverEntry>())
//     }
// }

impl HasResizeData for Synthetic<ResizeObserverEntry> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_border_box_size(&self) -> ResizeResult<PixelsSize> {
        extract_first_size(self.event.border_box_size())
    }

    fn get_content_box_size(&self) -> ResizeResult<PixelsSize> {
        extract_first_size(self.event.content_box_size())
    }
}

impl HasScrollData for Synthetic<Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasClipboardData for Synthetic<Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFocusData for Synthetic<web_sys::FocusEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasToggleData for Synthetic<web_sys::Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasSelectionData for Synthetic<web_sys::Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasMediaData for Synthetic<web_sys::Event> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFileData for Synthetic<web_sys::Event> {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn dioxus_html::FileEngine>> {
        let files = self
            .event
            .dyn_ref()
            .and_then(|input: &web_sys::HtmlInputElement| {
                input.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    super::file::WebFileEngine::new(files).map(|f| {
                        std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                    })
                })
            });

        files
    }
}
