use crate::events::{
    AnimationData, CompositionData, KeyboardData, MouseData, PointerData, TouchData,
    TransitionData, WheelData,
};
use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{decode_key_location, decode_mouse_button_set, MouseButton};
use crate::{DragData, MountedData};
use keyboard_types::{Code, Key, Modifiers};
use std::convert::TryInto;
use std::str::FromStr;
use wasm_bindgen::{JsCast, JsValue};
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
    MouseEvent       => DragData,
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

        Self::new(
            Key::from_str(&e.key()).expect("could not parse key"),
            Code::from_str(&e.code()).expect("could not parse code"),
            decode_key_location(
                e.location()
                    .try_into()
                    .expect("could not convert location to u32"),
            ),
            e.repeat(),
            modifiers,
        )
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
            Some(MouseButton::from_web_code(e.button())),
            decode_mouse_button_set(e.buttons()),
            modifiers,
        )
    }
}

impl From<&MouseEvent> for DragData {
    fn from(value: &MouseEvent) -> Self {
        Self {
            mouse: MouseData::from(value),
        }
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
        WheelData::from_web_attributes(e.delta_mode(), e.delta_x(), e.delta_y(), e.delta_z())
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

    fn get_raw_element(&self) -> crate::MountedResult<&dyn std::any::Any> {
        Ok(self)
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
