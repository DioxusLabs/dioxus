use dioxus_html::{
    geometry::{ClientPoint, ElementPoint, PagePoint, ScreenPoint},
    input_data::{decode_mouse_button_set, MouseButton},
    HasMouseData, HasWheelData, InteractionElementOffset, InteractionLocation, Modifiers,
    ModifiersInteraction, PointerInteraction,
};
use web_sys::WheelEvent;

use super::{Synthetic, WebEventExt};

impl HasWheelData for Synthetic<WheelEvent> {
    fn delta(&self) -> dioxus_html::geometry::WheelDelta {
        dioxus_html::geometry::WheelDelta::from_web_attributes(
            self.event.delta_mode(),
            self.event.delta_x(),
            self.event.delta_y(),
            self.event.delta_z(),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
    }
}

impl HasMouseData for Synthetic<WheelEvent> {
    fn as_any(&self) -> &dyn std::any::Any {
        &self.event
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
    fn held_buttons(&self) -> dioxus_html::input_data::MouseButtonSet {
        decode_mouse_button_set(self.event.buttons())
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.event.button()))
    }
}

impl WebEventExt for dioxus_html::WheelData {
    type WebEvent = web_sys::WheelEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<Self::WebEvent> {
        self.downcast::<web_sys::WheelEvent>().cloned()
    }
}
