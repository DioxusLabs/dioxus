use crate::events::HasKeyboardData;
use crate::events::{
    AnimationData, CompositionData, KeyboardData, MouseData, PointerData, TouchData,
    TransitionData, WheelData,
};
use crate::file_data::{FileEngine, HasFileData};
use crate::geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{decode_key_location, decode_mouse_button_set, MouseButton};
use crate::prelude::*;
use keyboard_types::{Code, Key, Modifiers};
use std::str::FromStr;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    AnimationEvent, CompositionEvent, Event, KeyboardEvent, MouseEvent, PointerEvent, Touch,
    TouchEvent, TransitionEvent, WheelEvent,
};

macro_rules! uncheck_convert {
    ($t:ty, $d:ty) => {
        impl From<Event> for $d {
            #[inline]
            fn from(e: Event) -> Self {
                let e: $t = e.unchecked_into();
                Self::from(e)
            }
        }

        impl From<&Event> for $d {
            #[inline]
            fn from(e: &Event) -> Self {
                let e: &$t = e.unchecked_ref();
                Self::from(e.clone())
            }
        }
    };
    ($($t:ty => $d:ty),+ $(,)?) => {
        $(uncheck_convert!($t, $d);)+
    };
}

uncheck_convert![
    web_sys::CompositionEvent => CompositionData,
    web_sys::KeyboardEvent    => KeyboardData,
    web_sys::MouseEvent       => MouseData,
    web_sys::TouchEvent       => TouchData,
    web_sys::PointerEvent     => PointerData,
    web_sys::WheelEvent       => WheelData,
    web_sys::AnimationEvent   => AnimationData,
    web_sys::TransitionEvent  => TransitionData,
    web_sys::MouseEvent       => DragData,
    web_sys::FocusEvent       => FocusData,
];

impl HasCompositionData for CompositionEvent {
    fn data(&self) -> std::string::String {
        self.data().unwrap_or_default()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasKeyboardData for KeyboardEvent {
    fn key(&self) -> Key {
        Key::from_str(self.key().as_str()).unwrap_or(Key::Unidentified)
    }

    fn code(&self) -> Code {
        Code::from_str(self.code().as_str()).unwrap_or(Code::Unidentified)
    }

    fn location(&self) -> keyboard_types::Location {
        decode_key_location(self.location() as usize)
    }

    fn is_auto_repeating(&self) -> bool {
        self.repeat()
    }

    fn is_composing(&self) -> bool {
        self.is_composing()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModifiersInteraction for KeyboardEvent {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl InteractionLocation for MouseEvent {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x().into(), self.client_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.page_x().into(), self.page_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.screen_x().into(), self.screen_y().into())
    }
}

impl InteractionElementOffset for MouseEvent {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.offset_x().into(), self.offset_y().into())
    }
}

impl ModifiersInteraction for MouseEvent {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl PointerInteraction for MouseEvent {
    fn held_buttons(&self) -> crate::input_data::MouseButtonSet {
        decode_mouse_button_set(self.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.button()))
    }
}

impl HasMouseData for MouseEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFileData for MouseEvent {}

impl HasDragData for MouseEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModifiersInteraction for TouchEvent {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl crate::events::HasTouchData for TouchEvent {
    fn touches(&self) -> Vec<TouchPoint> {
        let touches = TouchEvent::touches(self);
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(touch));
        }
        result
    }

    fn touches_changed(&self) -> Vec<TouchPoint> {
        let touches = self.changed_touches();
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(touch));
        }
        result
    }

    fn target_touches(&self) -> Vec<TouchPoint> {
        let touches = self.target_touches();
        let mut result = Vec::with_capacity(touches.length() as usize);
        for i in 0..touches.length() {
            let touch = touches.get(i).unwrap();
            result.push(TouchPoint::new(touch));
        }
        result
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasTouchPointData for Touch {
    fn identifier(&self) -> i32 {
        self.identifier()
    }

    fn radius(&self) -> ScreenPoint {
        ScreenPoint::new(self.radius_x().into(), self.radius_y().into())
    }

    fn rotation(&self) -> f64 {
        self.rotation_angle() as f64
    }

    fn force(&self) -> f64 {
        self.force() as f64
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for Touch {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x().into(), self.client_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.screen_x().into(), self.screen_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.page_x().into(), self.page_y().into())
    }
}

impl HasPointerData for PointerEvent {
    fn pointer_id(&self) -> i32 {
        self.pointer_id()
    }

    fn width(&self) -> i32 {
        self.width()
    }

    fn height(&self) -> i32 {
        self.height()
    }

    fn pressure(&self) -> f32 {
        self.pressure()
    }

    fn tangential_pressure(&self) -> f32 {
        self.tangential_pressure()
    }

    fn tilt_x(&self) -> i32 {
        self.tilt_x()
    }

    fn tilt_y(&self) -> i32 {
        self.tilt_y()
    }

    fn twist(&self) -> i32 {
        self.twist()
    }

    fn pointer_type(&self) -> String {
        self.pointer_type()
    }

    fn is_primary(&self) -> bool {
        self.is_primary()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl InteractionLocation for PointerEvent {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x().into(), self.client_y().into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.screen_x().into(), self.screen_y().into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.page_x().into(), self.page_y().into())
    }
}

impl InteractionElementOffset for PointerEvent {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.offset_x().into(), self.offset_y().into())
    }
}

impl ModifiersInteraction for PointerEvent {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        if self.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if self.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }
}

impl PointerInteraction for PointerEvent {
    fn held_buttons(&self) -> crate::input_data::MouseButtonSet {
        decode_mouse_button_set(self.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.button()))
    }
}

impl HasWheelData for WheelEvent {
    fn delta(&self) -> crate::geometry::WheelDelta {
        crate::geometry::WheelDelta::from_web_attributes(
            self.delta_mode(),
            self.delta_x(),
            self.delta_y(),
            self.delta_z(),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasAnimationData for AnimationEvent {
    fn animation_name(&self) -> String {
        self.animation_name()
    }

    fn pseudo_element(&self) -> String {
        self.pseudo_element()
    }

    fn elapsed_time(&self) -> f32 {
        self.elapsed_time()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasTransitionData for TransitionEvent {
    fn elapsed_time(&self) -> f32 {
        self.elapsed_time()
    }

    fn property_name(&self) -> String {
        self.property_name()
    }

    fn pseudo_element(&self) -> String {
        self.pseudo_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "mounted")]
impl From<&web_sys::Element> for MountedData {
    fn from(e: &web_sys::Element) -> Self {
        MountedData::new(e.clone())
    }
}

#[cfg(feature = "mounted")]
impl crate::RenderedElementBacking for web_sys::Element {
    fn get_client_rect(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::MountedResult<euclid::Rect<f64, f64>>>>,
    > {
        let rect = self.get_bounding_client_rect();
        let result = Ok(euclid::Rect::new(
            euclid::Point2D::new(rect.left(), rect.top()),
            euclid::Size2D::new(rect.width(), rect.height()),
        ));
        Box::pin(async { result })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn scroll_to(
        &self,
        behavior: crate::ScrollBehavior,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::MountedResult<()>>>> {
        match behavior {
            crate::ScrollBehavior::Instant => self.scroll_into_view_with_scroll_into_view_options(
                web_sys::ScrollIntoViewOptions::new().behavior(web_sys::ScrollBehavior::Instant),
            ),
            crate::ScrollBehavior::Smooth => self.scroll_into_view_with_scroll_into_view_options(
                web_sys::ScrollIntoViewOptions::new().behavior(web_sys::ScrollBehavior::Smooth),
            ),
        }

        Box::pin(async { Ok(()) })
    }

    fn set_focus(
        &self,
        focus: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::MountedResult<()>>>> {
        let result = self
            .dyn_ref::<web_sys::HtmlElement>()
            .ok_or_else(|| crate::MountedError::OperationFailed(Box::new(FocusError(self.into()))))
            .and_then(|e| {
                (if focus { e.focus() } else { e.blur() })
                    .map_err(|err| crate::MountedError::OperationFailed(Box::new(FocusError(err))))
            });
        Box::pin(async { result })
    }
}

#[derive(Debug)]
struct FocusError(JsValue);

impl std::fmt::Display for FocusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to focus element {:?}", self.0)
    }
}

impl std::error::Error for FocusError {}

impl HasScrollData for Event {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasClipboardData for Event {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<&Event> for ClipboardData {
    fn from(e: &Event) -> Self {
        ClipboardData::new(e.clone())
    }
}

impl HasFocusData for web_sys::FocusEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasToggleData for web_sys::Event {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasSelectionData for web_sys::Event {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasMediaData for web_sys::Event {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl HasFileData for web_sys::Event {
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        #[cfg(not(feature = "file_engine"))]
        let files = None;
        #[cfg(feature = "file_engine")]
        let files = element
            .dyn_ref()
            .and_then(|input: &web_sys::HtmlInputElement| {
                input.files().and_then(|files| {
                    #[allow(clippy::arc_with_non_send_sync)]
                    crate::file_engine::WebFileEngine::new(files).map(|f| {
                        std::sync::Arc::new(f) as std::sync::Arc<dyn dioxus_html::FileEngine>
                    })
                })
            });

        files
    }
}
