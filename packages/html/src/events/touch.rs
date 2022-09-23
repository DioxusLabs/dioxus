use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    TouchEvent: [
        /// ontouchcancel
        ontouchcancel

        /// ontouchend
        ontouchend

        /// ontouchmove
        ontouchmove

        /// ontouchstart
        ontouchstart
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    // get_modifier_state: bool,
    // changedTouches: DOMTouchList,
    // targetTouches: DOMTouchList,
    // touches: DOMTouchList,
}
