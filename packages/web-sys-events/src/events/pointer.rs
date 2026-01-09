use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{decode_mouse_button_set, MouseButton},
    HasPointerData, InteractionElementOffset, InteractionLocation, Modifiers, ModifiersInteraction,
    PointerInteraction,
};
use web_sys::PointerEvent;

use super::{Synthetic, WebEventExt};

impl HasPointerData for Synthetic<PointerEvent> {
    fn pointer_id(&self) -> i32 {
        self.event.pointer_id()
    }

    fn width(&self) -> f64 {
        self.event.width() as _
    }

    fn height(&self) -> f64 {
        self.event.height() as _
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
        &self.event
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
    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl WebEventExt for dioxus_html::PointerData {
    type WebEvent = web_sys::PointerEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::PointerEvent> {
        self.downcast::<web_sys::PointerEvent>().cloned()
    }
}
