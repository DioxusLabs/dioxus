use dioxus_html::{
    geometry::{ClientPoint, PagePoint, ScreenPoint},
    HasTouchPointData, InteractionLocation, Modifiers, ModifiersInteraction, TouchPoint,
};
use web_sys::{Touch, TouchEvent};

use super::{Synthetic, WebEventExt};

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

impl dioxus_html::events::HasTouchData for Synthetic<TouchEvent> {
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
        &self.event
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
        &self.event
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

impl WebEventExt for dioxus_html::TouchData {
    type WebEvent = web_sys::TouchEvent;

    #[inline(always)]
    fn try_as_web_event(&self) -> Option<web_sys::TouchEvent> {
        self.downcast::<web_sys::TouchEvent>().cloned()
    }
}
