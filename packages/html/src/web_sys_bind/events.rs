use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input::{decode_mouse_button_set, MouseButton};
use crate::on::{
    AnimationData, CompositionData, KeyboardData, MouseData, PointerData, TouchData,
    TransitionData, WheelData,
};
use crate::KeyCode;
use keyboard_types::Modifiers;
use wasm_bindgen::JsCast;
use web_sys::{
    AnimationEvent, CompositionEvent, Event, KeyboardEvent, MouseEvent, PointerEvent, TouchEvent,
    TransitionEvent, WheelEvent,
};

macro_rules! uncheck_convert {
    ($t:ty, $d:ty) => {
        impl From<Event> for $d {
            #[inline]
            fn from(e: Event) -> Self {
                let e: $t = e.unchecked_into();
                Self::from(&e)
            }
        }

        impl From<&Event> for $d {
            #[inline]
            fn from(e: &Event) -> Self {
                let e: &$t = e.unchecked_ref();
                Self::from(e)
            }
        }
    };
    ($($t:ty => $d:ty),+ $(,)?) => {
        $(uncheck_convert!($t, $d);)+
    };
}

uncheck_convert![
    CompositionEvent => CompositionData,
    KeyboardEvent    => KeyboardData,
    MouseEvent       => MouseData,
    TouchEvent       => TouchData,
    PointerEvent     => PointerData,
    WheelEvent       => WheelData,
    AnimationEvent   => AnimationData,
    TransitionEvent  => TransitionData,
];

impl From<&CompositionEvent> for CompositionData {
    fn from(e: &CompositionEvent) -> Self {
        Self {
            data: e.data().unwrap_or_default(),
        }
    }
}

impl From<&KeyboardEvent> for KeyboardData {
    fn from(e: &KeyboardEvent) -> Self {
        Self {
            alt_key: e.alt_key(),
            char_code: e.char_code(),
            key: e.key(),
            key_code: KeyCode::from_raw_code(e.key_code() as u8),
            ctrl_key: e.ctrl_key(),
            locale: "not implemented".to_string(),
            location: e.location() as usize,
            meta_key: e.meta_key(),
            repeat: e.repeat(),
            shift_key: e.shift_key(),
            which: e.which() as usize,
        }
    }
}

impl From<&MouseEvent> for MouseData {
    fn from(e: &MouseEvent) -> Self {
        let mut modifiers = Modifiers::empty();

        if e.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if e.ctrl_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if e.meta_key() {
            modifiers.insert(Modifiers::META);
        }
        if e.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }

        MouseData::new(
            Coordinates::new(
                ScreenPoint::new(e.screen_x().into(), e.screen_y().into()),
                ClientPoint::new(e.client_x().into(), e.client_y().into()),
                ElementPoint::new(e.offset_x().into(), e.offset_y().into()),
                PagePoint::new(e.page_x().into(), e.page_y().into()),
            ),
            Some(MouseButton::from_web_code(e.button().into())),
            decode_mouse_button_set(e.buttons()),
            modifiers,
        )
    }
}

impl From<&TouchEvent> for TouchData {
    fn from(e: &TouchEvent) -> Self {
        Self {
            alt_key: e.alt_key(),
            ctrl_key: e.ctrl_key(),
            meta_key: e.meta_key(),
            shift_key: e.shift_key(),
        }
    }
}

impl From<&PointerEvent> for PointerData {
    fn from(e: &PointerEvent) -> Self {
        Self {
            alt_key: e.alt_key(),
            button: e.button(),
            buttons: e.buttons(),
            client_x: e.client_x(),
            client_y: e.client_y(),
            ctrl_key: e.ctrl_key(),
            meta_key: e.meta_key(),
            page_x: e.page_x(),
            page_y: e.page_y(),
            screen_x: e.screen_x(),
            screen_y: e.screen_y(),
            shift_key: e.shift_key(),
            pointer_id: e.pointer_id(),
            width: e.width(),
            height: e.height(),
            pressure: e.pressure(),
            tangential_pressure: e.tangential_pressure(),
            tilt_x: e.tilt_x(),
            tilt_y: e.tilt_y(),
            twist: e.twist(),
            pointer_type: e.pointer_type(),
            is_primary: e.is_primary(),
            // get_modifier_state: evt.get_modifier_state(),
        }
    }
}

impl From<&WheelEvent> for WheelData {
    fn from(e: &WheelEvent) -> Self {
        Self {
            delta_x: e.delta_x(),
            delta_y: e.delta_y(),
            delta_z: e.delta_z(),
            delta_mode: e.delta_mode(),
        }
    }
}

impl From<&AnimationEvent> for AnimationData {
    fn from(e: &AnimationEvent) -> Self {
        Self {
            elapsed_time: e.elapsed_time(),
            animation_name: e.animation_name(),
            pseudo_element: e.pseudo_element(),
        }
    }
}

impl From<&TransitionEvent> for TransitionData {
    fn from(e: &TransitionEvent) -> Self {
        Self {
            elapsed_time: e.elapsed_time(),
            property_name: e.property_name(),
            pseudo_element: e.pseudo_element(),
        }
    }
}
