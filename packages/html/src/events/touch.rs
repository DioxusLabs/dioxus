use dioxus_core::Event;

pub type TouchEvent = Event<TouchData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TouchData {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    // get_modifier_state: bool,
    // changedTouches: DOMTouchList,
    // targetTouches: DOMTouchList,
    // touches: DOMTouchList,
}

impl_event! {
    TouchData;
    /// touchstart
    ontouchstart

    /// touchmove
    ontouchmove

    /// touchend
    ontouchend

    /// touchcancel
    ontouchcancel
}
